use std::{collections::HashSet, future::IntoFuture, sync::Arc, time::Instant};

use ahash::AHashMap;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing};
use qdrant_client::qdrant::{
	CreateFieldIndexCollection, FieldType, GetPointsBuilder, PayloadSchemaType, RetrievedPoint,
	value,
};
use serde_json::Map;
use sqlx::{FromRow, PgPool};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
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
	ElfService, Error, Providers, TextQuoteSelector,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker;

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

async fn wait_for_doc_outbox_done(
	pool: &PgPool,
	doc_id: Uuid,
	timeout: std::time::Duration,
) -> bool {
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

		tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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
		wait_for_doc_outbox_done(&service.db.pool, put.doc_id, std::time::Duration::from_secs(15))
			.await,
		"Expected doc outbox to reach DONE."
	);

	assert_docs_search_l0(&service, put.doc_id).await;

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_respects_scope_doc_type_agent_id_and_updated_after_filters() {
	let Some(ctx) = setup_docs_context().await else { return };
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
			private_chat_doc.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected private docs outbox to reach DONE."
	);

	let shared_scope_results = search_doc_ids_with_filters(
		&service,
		Some("project_shared"),
		None,
		None,
		None,
		None,
		"reader",
	)
	.await;

	assert!(shared_scope_results.contains(&shared_knowledge_doc.doc_id));
	assert!(!shared_scope_results.contains(&private_chat_doc.doc_id));

	let chat_results =
		search_doc_ids_with_filters(&service, None, Some("chat"), None, None, None, "reader").await;

	assert!(!chat_results.contains(&private_chat_doc.doc_id));
	assert!(!chat_results.contains(&shared_knowledge_doc.doc_id));

	let assistant_chat_results =
		search_doc_ids_with_filters(&service, None, Some("chat"), None, None, None, "assistant")
			.await;

	assert!(assistant_chat_results.contains(&private_chat_doc.doc_id));
	assert!(!assistant_chat_results.contains(&shared_knowledge_doc.doc_id));

	let assistant_results =
		search_doc_ids_with_filters(&service, None, None, Some("assistant"), None, None, "reader")
			.await;

	assert!(!assistant_results.contains(&private_chat_doc.doc_id));
	assert!(!assistant_results.contains(&shared_knowledge_doc.doc_id));

	let past = (OffsetDateTime::now_utc() - time::Duration::seconds(60))
		.format(&Rfc3339)
		.expect("Failed to format past RFC3339 timestamp.");
	let future = (OffsetDateTime::now_utc() + time::Duration::seconds(60))
		.format(&Rfc3339)
		.expect("Failed to format future RFC3339 timestamp.");
	let updated_after_past_results =
		search_doc_ids_with_filters(&service, None, None, None, Some(&past), None, "reader").await;

	assert!(updated_after_past_results.contains(&shared_knowledge_doc.doc_id));
	assert!(!updated_after_past_results.contains(&private_chat_doc.doc_id));

	let updated_after_future_results =
		search_doc_ids_with_filters(&service, None, None, None, Some(&future), None, "reader")
			.await;

	assert!(updated_after_future_results.is_empty());

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_put_rejects_non_english_source_ref() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping docs_extension_v1; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping docs_extension_v1; set ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."
		);

		return;
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
	let result = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs rejection sample".to_string()),
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"notes": "你好"
			}),
			content: TEST_CONTENT.to_string(),
		})
		.await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.source_ref[\"notes\"]");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_put_rejects_missing_and_invalid_source_ref() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping docs_extension_v1; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping docs_extension_v1; set ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."
		);

		return;
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
	let result = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs rejection sample".to_string()),
			source_ref: serde_json::json!("legacy-shape"),
			content: TEST_CONTENT.to_string(),
		})
		.await;

	match result {
		Err(Error::InvalidRequest { message }) => {
			assert!(message.contains("source_ref must be a JSON object"));
		},
		other => panic!("Expected InvalidRequest for non-object source_ref, got {other:?}"),
	}

	let result = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs rejection sample".to_string()),
			source_ref: serde_json::json!({
				"schema": "source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: TEST_CONTENT.to_string(),
		})
		.await;

	match result {
		Err(Error::InvalidRequest { message }) => {
			assert!(message.contains("doc_source_ref/v1"));
		},
		other => panic!("Expected InvalidRequest for wrong source_ref schema, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_requires_qdrant_payload_indexes_for_filters() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let doc = put_test_doc(&service).await;
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(&service.db.pool, doc.doc_id, std::time::Duration::from_secs(15))
			.await,
		"Expected doc outbox to reach DONE."
	);

	verify_docs_qdrant_filter_indexes(&service).await;

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_projects_source_ref_payload_fields() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let source_ts = "2025-01-01T10:00:00Z";
	let cases = [
		(
			"chat",
			"Docs chat source ref sample",
			serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "chat",
				"ts": source_ts,
				"thread_id": "thread-42",
				"role": "assistant"
			}),
			("thread_id", "thread-42"),
			["domain", "repo"],
		),
		(
			"search",
			"Docs search source ref sample",
			serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "search",
				"ts": source_ts,
				"query": "What is payload indexing?",
				"url": "https://docs.example.com/search",
				"domain": "docs.example.com",
				"provider": "web"
			}),
			("domain", "docs.example.com"),
			["thread_id", "repo"],
		),
		(
			"dev",
			"Docs dev source ref sample",
			serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "dev",
				"ts": source_ts,
				"repo": "elf-org/docs",
				"commit_sha": "9f0a3f4c4eb58bfcf4a5f4f9d0c7be0e13c2f8d19"
			}),
			("repo", "elf-org/docs"),
			["thread_id", "domain"],
		),
	];
	let mut docs = Vec::new();

	for (doc_type, title, source_ref, expected_present, expected_absent) in cases {
		let doc = put_test_doc_with(
			&service,
			"owner",
			"project_shared",
			Some(doc_type),
			title,
			source_ref,
			TEST_CONTENT,
		)
		.await;

		docs.push((doc.doc_id, expected_present, expected_absent));
	}

	let (handle, shutdown) = spawn_doc_worker(&service).await;

	for (doc_id, expected_present, expected_absent) in &docs {
		assert!(
			wait_for_doc_outbox_done(&service.db.pool, *doc_id, std::time::Duration::from_secs(15))
				.await,
			"Expected doc outbox to reach DONE."
		);

		let point = fetch_first_doc_chunk_point(&service, *doc_id)
			.await
			.expect("Expected doc chunk point in Qdrant.");

		assert_eq!(point.payload.get("doc_ts").and_then(payload_string), Some(source_ts));
		assert_eq!(
			point.payload.get(expected_present.0).and_then(payload_string),
			Some(expected_present.1)
		);

		for key in expected_absent {
			assert!(!point.payload.contains_key(*key));
		}
	}

	_ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

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
			doc_type: doc_type.map(std::string::ToString::to_string),
			title: Some(title.to_string()),
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
			agent_id: agent_id.map(str::to_string),
			updated_after: updated_after.map(str::to_string),
			updated_before: updated_before.map(str::to_string),
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
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
