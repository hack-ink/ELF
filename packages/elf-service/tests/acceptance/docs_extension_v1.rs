mod excerpts;
mod indexing;
mod l0_search;
mod lifecycle;
mod search_filters;
mod validation_rejections;

use std::{
	collections::HashSet,
	future::IntoFuture,
	string::ToString,
	sync::Arc,
	time::{Duration, Instant},
};

use ahash::AHashMap;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing};
use qdrant_client::qdrant::{
	CreateFieldIndexCollection, FieldType, GetPointsBuilder, PayloadSchemaType, RetrievedPoint,
	value,
};
use serde_json::Map;
use sqlx::{FromRow, PgPool};
use tokenizers::{Tokenizer, models::wordlevel::WordLevel};
use tokio::{
	net::TcpListener,
	sync::{oneshot, oneshot::Sender},
	task::JoinHandle,
	time,
};
use uuid::Uuid;

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank, chunking::ChunkingConfig};
use elf_config::EmbeddingProviderConfig;
use elf_service::{
	DocsExcerptsGetRequest, DocsGetRequest, DocsPutRequest, DocsPutResponse, DocsSearchL0Request,
	ElfService, Providers, TextQuoteSelector, docs::DocRetrievalTrajectory,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker::{self, WorkerState};

const TEST_CONTENT: &str =
	"ELF docs extension v1 stores evidence. Keyword: peregrine.\nSecond sentence for chunking.";
const DOCS_SEARCH_FILTER_INDEXES: [(&str, PayloadSchemaType, FieldType); 9] = [
	("scope", PayloadSchemaType::Keyword, FieldType::Keyword),
	("status", PayloadSchemaType::Keyword, FieldType::Keyword),
	("doc_type", PayloadSchemaType::Keyword, FieldType::Keyword),
	("agent_id", PayloadSchemaType::Keyword, FieldType::Keyword),
	("updated_at", PayloadSchemaType::Datetime, FieldType::Datetime),
	("doc_ts", PayloadSchemaType::Datetime, FieldType::Datetime),
	("thread_id", PayloadSchemaType::Keyword, FieldType::Keyword),
	("domain", PayloadSchemaType::Keyword, FieldType::Keyword),
	("repo", PayloadSchemaType::Keyword, FieldType::Keyword),
];

#[derive(FromRow)]
struct DocOutboxCounts {
	total: i64,
	done: i64,
	failed: i64,
}

#[derive(FromRow)]
struct NoteOutboxCounts {
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

fn payload_string(payload_value: &qdrant_client::qdrant::Value) -> Option<&str> {
	match payload_value.kind.as_ref() {
		Some(value::Kind::StringValue(value)) => Some(value.as_str()),
		_ => None,
	}
}

fn trajectory_stage_stats<'a>(
	trajectory: &'a DocRetrievalTrajectory,
	stage_name: &str,
) -> Option<&'a serde_json::Value> {
	trajectory.stages.iter().find(|stage| stage.stage_name == stage_name).map(|stage| &stage.stats)
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

		time::sleep(Duration::from_millis(200)).await;
	}
}

async fn wait_for_note_outbox_done(pool: &PgPool, note_id: Uuid, timeout: Duration) -> bool {
	let deadline = Instant::now() + timeout;

	loop {
		let row: Option<NoteOutboxCounts> = sqlx::query_as::<_, NoteOutboxCounts>(
			"\
SELECT
	COUNT(*) AS total,
	COUNT(*) FILTER (WHERE status = 'DONE') AS done,
	COUNT(*) FILTER (WHERE status = 'FAILED') AS failed
FROM indexing_outbox
WHERE note_id = $1",
		)
		.bind(note_id)
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

		time::sleep(Duration::from_millis(200)).await;
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

async fn embed_handler(
	State(()): State<()>,
	Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
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

async fn create_docs_search_filter_fixture(
	ctx: DocsContext,
) -> (TestDatabase, ElfService, Uuid, Uuid, Uuid, JoinHandle<()>, Sender<()>) {
	let DocsContext { test_db, service } = ctx;
	let shared_knowledge_doc = put_test_doc_with(
		&service,
		"owner",
		"project_shared",
		None,
		"Docs filter sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let older_shared_knowledge_doc = put_test_doc_with(
		&service,
		"owner",
		"project_shared",
		None,
		"Docs old filter sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2025-01-01T10:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let private_chat_doc = put_test_doc_with(
		&service,
		"assistant",
		"agent_private",
		Some("chat"),
		"Docs chat sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "chat",
			"ts": "2026-02-25T12:00:00Z",
			"thread_id": "shared-chat-thread",
			"role": "assistant"
		}),
		TEST_CONTENT,
	)
	.await;
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(
			&service.db.pool,
			shared_knowledge_doc.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected shared docs outbox to reach DONE."
	);
	assert!(
		wait_for_doc_outbox_done(
			&service.db.pool,
			older_shared_knowledge_doc.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected older shared docs outbox to reach DONE."
	);
	assert!(
		wait_for_doc_outbox_done(
			&service.db.pool,
			private_chat_doc.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected private docs outbox to reach DONE."
	);

	(
		test_db,
		service,
		shared_knowledge_doc.doc_id,
		older_shared_knowledge_doc.doc_id,
		private_chat_doc.doc_id,
		handle,
		shutdown,
	)
}

async fn cleanup_docs_filter_fixture(
	test_db: TestDatabase,
	_handle: JoinHandle<()>,
	shutdown: Sender<()>,
) {
	let _ = shutdown.send(());

	_handle.abort();

	let _ = _handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn setup_docs_context() -> Option<DocsContext> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping docs_extension_v1; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping docs_extension_v1; set ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."
		);

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = acceptance::test_config(
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
		acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.qdrant.collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant memory collection.");
	acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.cfg.storage.qdrant.docs_collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant docs collection.");

	Some(DocsContext { test_db, service })
}

async fn fetch_first_doc_chunk_id(db: &ElfService, doc_id: Uuid) -> Option<Uuid> {
	sqlx::query_scalar::<_, Uuid>(
		"SELECT chunk_id FROM doc_chunks WHERE doc_id = $1 ORDER BY chunk_index LIMIT 1",
	)
	.bind(doc_id)
	.fetch_optional(&db.db.pool)
	.await
	.expect("Failed to fetch doc chunk id.")
}

async fn fetch_first_doc_chunk_point(service: &ElfService, doc_id: Uuid) -> Option<RetrievedPoint> {
	let chunk_id = fetch_first_doc_chunk_id(service, doc_id).await?;
	let response = service
		.qdrant
		.client
		.get_points(
			GetPointsBuilder::new(
				service.cfg.storage.qdrant.docs_collection.clone(),
				vec![chunk_id.to_string().into()],
			)
			.with_payload(true),
		)
		.await
		.expect("Failed to fetch doc chunk point from Qdrant.");

	response.result.into_iter().next()
}

async fn put_test_doc(service: &ElfService) -> DocsPutResponse {
	put_test_doc_with(
		service,
		"owner",
		"project_shared",
		None,
		"Docs v1",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"uri": "acceptance://knowledge/v1"
		}),
		TEST_CONTENT,
	)
	.await
}

async fn put_test_doc_with(
	service: &ElfService,
	agent_id: &str,
	scope: &str,
	doc_type: Option<&str>,
	title: &str,
	source_ref: serde_json::Value,
	content: &str,
) -> DocsPutResponse {
	service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: agent_id.to_string(),
			scope: scope.to_string(),
			doc_type: doc_type.map(ToString::to_string),
			title: Some(title.to_string()),
			write_policy: None,
			source_ref,
			content: content.to_string(),
		})
		.await
		.expect("Failed to put doc.")
}

async fn search_doc_ids_with_filters(
	service: &ElfService,
	scope: Option<&str>,
	doc_type: Option<&str>,
	agent_id: Option<&str>,
	updated_after: Option<&str>,
	updated_before: Option<&str>,
	caller_agent_id: &str,
) -> HashSet<Uuid> {
	let results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: caller_agent_id.to_string(),
			scope: scope.map(str::to_string),
			status: None,
			doc_type: doc_type.map(str::to_string),
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: agent_id.map(str::to_string),
			thread_id: None,
			updated_after: updated_after.map(str::to_string),
			updated_before: updated_before.map(str::to_string),
			ts_gte: None,
			ts_lte: None,
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs.");

	results.items.into_iter().map(|item| item.doc_id).collect()
}

async fn verify_docs_qdrant_filter_indexes(service: &ElfService) {
	let mut payload_schema = service
		.qdrant
		.client
		.collection_info(&service.cfg.storage.qdrant.docs_collection)
		.await
		.expect("Failed to fetch Qdrant docs collection info.")
		.result
		.expect("Qdrant collection info is missing.")
		.payload_schema;

	for (field_name, payload_type, index_type) in DOCS_SEARCH_FILTER_INDEXES {
		let missing_or_wrong = match payload_schema.get(field_name) {
			Some(schema) => schema.data_type != payload_type as i32,
			None => true,
		};

		if missing_or_wrong {
			let request = CreateFieldIndexCollection {
				collection_name: service.cfg.storage.qdrant.docs_collection.clone(),
				wait: Some(true),
				field_name: field_name.to_string(),
				field_type: Some(index_type as i32),
				field_index_params: None,
				ordering: None,
				timeout: None,
			};

			service
				.qdrant
				.client
				.create_field_index(request)
				.await
				.expect("Failed to create required Qdrant payload index.");
		}
	}

	payload_schema = service
		.qdrant
		.client
		.collection_info(&service.cfg.storage.qdrant.docs_collection)
		.await
		.expect("Failed to fetch Qdrant docs collection info.")
		.result
		.expect("Qdrant collection info is missing.")
		.payload_schema;

	for (field_name, payload_type, _) in DOCS_SEARCH_FILTER_INDEXES {
		let schema = payload_schema.get(field_name).expect("Expected required payload field.");

		assert_eq!(
			schema.data_type, payload_type as i32,
			"Unexpected payload type for {field_name}."
		);
	}
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
			explain: None,
		})
		.await
		.expect("Failed to get excerpt.");

	assert!(excerpts.verification.verified);
	assert!(excerpts.excerpt.contains("Keyword: peregrine."));
	assert_eq!(excerpts.verification.content_hash, content_hash);
}

async fn spawn_doc_worker(service: &ElfService) -> (JoinHandle<()>, Sender<()>) {
	let (api_base, shutdown) = start_embed_server().await;
	let worker_state = WorkerState {
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
		chunking: ChunkingConfig { max_tokens: 64, overlap_tokens: 8 },
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
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(5),
			candidate_k: Some(20),
			explain: None,
		})
		.await
		.expect("Failed to search docs.");

	assert!(!results.items.is_empty());
	assert_eq!(results.items[0].doc_id, doc_id);
	assert_eq!(results.items[0].doc_type, "knowledge");
	assert!(results.items[0].snippet.contains("peregrine"));
}
