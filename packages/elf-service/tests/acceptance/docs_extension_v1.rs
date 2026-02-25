use std::{
	future::IntoFuture,
	sync::Arc,
	time::{Duration, Instant},
};

use ahash::AHashMap;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing};
use serde_json::{Map, Value};
use sqlx::{FromRow, PgPool};
use tokenizers::{Tokenizer, models::wordlevel::WordLevel};
use tokio::{
	net::TcpListener,
	sync::{oneshot, oneshot::Sender},
	task::JoinHandle,
};
use uuid::Uuid;

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
use elf_config::EmbeddingProviderConfig;
use elf_service::{
	DocsExcerptsGetRequest, DocsGetRequest, DocsPutRequest, DocsPutResponse, DocsSearchL0Request,
	ElfService, Providers, TextQuoteSelector,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker;

const TEST_CONTENT: &str =
	"ELF docs extension v1 stores evidence. Keyword: peregrine.\nSecond sentence for chunking.";

#[derive(FromRow)]
struct DocOutboxCounts {
	total: i64,
	done: i64,
	failed: i64,
}

struct DocsContext {
	test_db: TestDatabase,
	service: ElfService,
}

fn build_test_tokenizer() -> Tokenizer {
	let mut vocab = AHashMap::new();

	vocab.insert("<unk>".to_string(), 0_u32);

	let model = WordLevel::builder()
		.vocab(vocab)
		.unk_token("<unk>".to_string())
		.build()
		.expect("Failed to build test tokenizer.");

	Tokenizer::new(model)
}

async fn wait_for_doc_outbox_done(pool: &PgPool, doc_id: Uuid, timeout: Duration) -> bool {
	let deadline = Instant::now() + timeout;

	loop {
		let row: Option<DocOutboxCounts> = sqlx::query_as::<_, DocOutboxCounts>(
			"\
SELECT
	COUNT(*) AS total,
	COUNT(*) FILTER (WHERE status = 'DONE') AS done,
	COUNT(*) FILTER (WHERE status = 'FAILED') AS failed
FROM doc_indexing_outbox
WHERE doc_id = $1",
		)
		.bind(doc_id)
		.fetch_optional(pool)
		.await
		.ok()
		.flatten();

		if let Some(row) = row.as_ref()
			&& row.total > 0
			&& row.done == row.total
		{
			return true;
		}
		if let Some(row) = row.as_ref()
			&& row.failed > 0
		{
			return false;
		}

		if Instant::now() >= deadline {
			return false;
		}

		tokio::time::sleep(Duration::from_millis(200)).await;
	}
}

async fn start_embed_server() -> (String, Sender<()>) {
	let app = Router::new().route("/embeddings", routing::post(embed_handler)).with_state(());
	let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind embed server.");
	let addr = listener.local_addr().expect("Failed to read embed server address.");
	let (tx, rx) = oneshot::channel();
	let server = axum::serve(listener, app).with_graceful_shutdown(async move {
		let _ = rx.await;
	});

	tokio::spawn(async move {
		let _ = server.into_future().await;
	});

	(format!("http://{addr}"), tx)
}

async fn embed_handler(State(()): State<()>, Json(payload): Json<Value>) -> impl IntoResponse {
	let inputs =
		payload.get("input").and_then(|value| value.as_array()).cloned().unwrap_or_default();
	let data: Vec<_> = inputs
		.iter()
		.enumerate()
		.map(|(index, _)| {
			let embedding: Vec<f32> = vec![0.1_f32; 4_096];

			serde_json::json!({
				"index": index,
				"embedding": embedding,
			})
		})
		.collect();

	(StatusCode::OK, Json(serde_json::json!({ "data": data }))).into_response()
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_put_get_excerpts_and_search_l0_work_end_to_end() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let put = put_test_doc(&service).await;

	assert_doc_get(&service, put.doc_id).await;
	assert_doc_excerpt(&service, put.doc_id, put.content_hash.as_str()).await;

	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(&service.db.pool, put.doc_id, Duration::from_secs(15)).await,
		"Expected doc outbox to reach DONE."
	);

	assert_docs_search_l0(&service, put.doc_id).await;

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn setup_docs_context() -> Option<DocsContext> {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping docs_extension_v1; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping docs_extension_v1; set ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."
		);

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = crate::acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(Default::default()),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	crate::acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.qdrant.collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant memory collection.");
	crate::acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.cfg.storage.qdrant.docs_collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant docs collection.");

	Some(DocsContext { test_db, service })
}

async fn put_test_doc(service: &ElfService) -> DocsPutResponse {
	service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs v1".to_string()),
			source_ref: serde_json::json!({ "source": "acceptance-test", "type": "text" }),
			content: TEST_CONTENT.to_string(),
		})
		.await
		.expect("Failed to put doc.")
}

async fn assert_doc_get(service: &ElfService, doc_id: Uuid) {
	let get_as_owner = service
		.docs_get(DocsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id,
		})
		.await
		.expect("Failed to get doc as owner.");

	assert_eq!(get_as_owner.scope, "project_shared");
	assert_eq!(get_as_owner.doc_type, "knowledge");
	assert_eq!(get_as_owner.agent_id, "owner");
	assert_eq!(get_as_owner.title.as_deref(), Some("Docs v1"));

	let get_as_reader = service
		.docs_get(DocsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id,
		})
		.await
		.expect("Failed to get doc as reader (expected project grant).");

	assert_eq!(get_as_reader.doc_id, doc_id);
}

async fn assert_doc_excerpt(service: &ElfService, doc_id: Uuid, content_hash: &str) {
	let excerpts = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id,
			level: "L1".to_string(),
			chunk_id: None,
			quote: Some(TextQuoteSelector {
				exact: "Keyword: peregrine.".to_string(),
				prefix: Some("evidence. ".to_string()),
				suffix: Some("\nSecond".to_string()),
			}),
			position: None,
		})
		.await
		.expect("Failed to get excerpt.");

	assert!(excerpts.verification.verified);
	assert!(excerpts.excerpt.contains("Keyword: peregrine."));
	assert_eq!(excerpts.verification.content_hash, content_hash);
}

async fn spawn_doc_worker(service: &ElfService) -> (JoinHandle<()>, Sender<()>) {
	let (api_base, shutdown) = start_embed_server().await;
	let worker_state = worker::WorkerState {
		db: Db::connect(&service.cfg.storage.postgres).await.expect("Failed to connect worker DB."),
		qdrant: QdrantStore::new(&service.cfg.storage.qdrant)
			.expect("Failed to build Qdrant store."),
		docs_qdrant: QdrantStore::new_with_collection(
			&service.cfg.storage.qdrant,
			&service.cfg.storage.qdrant.docs_collection,
		)
		.expect("Failed to build docs Qdrant store."),
		embedding: EmbeddingProviderConfig {
			provider_id: "test".to_string(),
			api_base,
			api_key: "test-key".to_string(),
			path: "/embeddings".to_string(),
			model: "test".to_string(),
			dimensions: 4_096,
			timeout_ms: 1_000,
			default_headers: Map::new(),
		},
		chunking: crate::acceptance::chunking::ChunkingConfig { max_tokens: 64, overlap_tokens: 8 },
		tokenizer: build_test_tokenizer(),
	};
	let handle = tokio::spawn(async move {
		let _ = worker::run_worker(worker_state).await;
	});

	(handle, shutdown)
}

async fn assert_docs_search_l0(service: &ElfService, doc_id: Uuid) {
	let results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			agent_id: None,
			updated_after: None,
			updated_before: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(5),
			candidate_k: Some(20),
		})
		.await
		.expect("Failed to search docs.");

	assert!(!results.items.is_empty());
	assert_eq!(results.items[0].doc_id, doc_id);
	assert_eq!(results.items[0].doc_type, "knowledge");
	assert!(results.items[0].snippet.contains("peregrine"));
}
