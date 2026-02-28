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
use tokio::{net::TcpListener, sync::oneshot::Sender, task::JoinHandle};
use uuid::Uuid;

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
use elf_config::EmbeddingProviderConfig;
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, DocsExcerptsGetRequest, DocsGetRequest,
	DocsPutRequest, DocsPutResponse, DocsSearchL0Request, ElfService, EmbeddingProvider, Error,
	Providers, Result, SearchRequest, TextQuoteSelector, docs::DocRetrievalTrajectory,
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

struct NonZeroSearchEmbedding;
impl EmbeddingProvider for NonZeroSearchEmbedding {
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		let vector = vec![0.1_f32; cfg.dimensions as usize];

		Box::pin(async move { Ok(vec![vector; texts.len()]) })
	}
}

struct DocsFilterFixtureIds {
	search_domain_doc_id: Uuid,
	search_other_domain_doc_id: Uuid,
	repo_doc_id: Uuid,
	repo_other_doc_id: Uuid,
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

fn configure_recency_bias_settings(service: &mut ElfService) {
	service.providers.embedding = Arc::new(NonZeroSearchEmbedding);
	service.cfg.ranking.tie_breaker_weight = 1_000.0;
	service.cfg.ranking.recency_tau_days = 36_500.0;
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

async fn wait_for_note_outbox_done(
	pool: &PgPool,
	note_id: Uuid,
	timeout: std::time::Duration,
) -> bool {
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

		tokio::time::sleep(std::time::Duration::from_millis(200)).await;
	}
}

async fn start_embed_server() -> (String, Sender<()>) {
	let app = Router::new().route("/embeddings", routing::post(embed_handler)).with_state(());
	let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind embed server.");
	let addr = listener.local_addr().expect("Failed to read embed server address.");
	let (tx, rx) = tokio::sync::oneshot::channel();
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
	let (
		test_db,
		service,
		shared_knowledge_doc,
		_older_shared_knowledge_doc,
		private_chat_doc,
		handle,
		shutdown,
	) = create_docs_search_filter_fixture(ctx).await;
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

	assert!(shared_scope_results.contains(&shared_knowledge_doc));
	assert!(!shared_scope_results.contains(&private_chat_doc));

	let chat_results =
		search_doc_ids_with_filters(&service, None, Some("chat"), None, None, None, "reader").await;

	assert!(!chat_results.contains(&private_chat_doc));
	assert!(!chat_results.contains(&shared_knowledge_doc));

	let assistant_chat_results =
		search_doc_ids_with_filters(&service, None, Some("chat"), None, None, None, "assistant")
			.await;

	assert!(assistant_chat_results.contains(&private_chat_doc));
	assert!(!assistant_chat_results.contains(&shared_knowledge_doc));

	let assistant_results =
		search_doc_ids_with_filters(&service, None, None, Some("assistant"), None, None, "reader")
			.await;

	assert!(!assistant_results.contains(&private_chat_doc));
	assert!(!assistant_results.contains(&shared_knowledge_doc));

	let past = (OffsetDateTime::now_utc() - time::Duration::seconds(60))
		.format(&Rfc3339)
		.expect("Failed to format past RFC3339 timestamp.");
	let future = (OffsetDateTime::now_utc() + time::Duration::seconds(60))
		.format(&Rfc3339)
		.expect("Failed to format future RFC3339 timestamp.");
	let updated_after_past_results =
		search_doc_ids_with_filters(&service, None, None, None, Some(&past), None, "reader").await;

	assert!(updated_after_past_results.contains(&shared_knowledge_doc));
	assert!(!updated_after_past_results.contains(&private_chat_doc));

	let updated_after_future_results =
		search_doc_ids_with_filters(&service, None, None, None, Some(&future), None, "reader")
			.await;

	assert!(updated_after_future_results.is_empty());

	cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_respects_thread_id_filter_for_chat_docs() {
	let Some(ctx) = setup_docs_context().await else { return };
	let (
		test_db,
		service,
		shared_knowledge_doc,
		older_shared_knowledge_doc,
		private_chat_doc,
		handle,
		shutdown,
	) = create_docs_search_filter_fixture(ctx).await;
	let thread_filter_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "assistant".to_string(),
			scope: None,
			status: None,
			doc_type: Some("chat".to_string()),
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: Some("shared-chat-thread".to_string()),
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs with thread_id filter.");
	let thread_filtered_docs =
		thread_filter_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(thread_filtered_docs.contains(&private_chat_doc));
	assert!(!thread_filtered_docs.contains(&shared_knowledge_doc));
	assert!(!thread_filtered_docs.contains(&older_shared_knowledge_doc));

	cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_requires_chat_doc_type_for_thread_id() {
	let Some(ctx) = setup_docs_context().await else { return };
	let (
		test_db,
		service,
		_shared_knowledge_doc,
		_older_shared_knowledge_doc,
		_private_chat_doc,
		handle,
		shutdown,
	) = create_docs_search_filter_fixture(ctx).await;
	let result = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "assistant".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: Some("shared-chat-thread".to_string()),
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await;

	match result {
		Err(Error::InvalidRequest { message }) => {
			assert!(message.contains("thread_id requires"));
		},
		other =>
			panic!("Expected InvalidRequest for thread_id without chat doc_type, got {other:?}"),
	}

	cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_put_applies_write_policy_and_excerpt_by_chunk_id_is_verified() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let content = "Alpha normal text then secret sk-abcdef and trailing content.";
	let secret = "sk-abcdef";
	let start = content.find(secret).expect("Expected secret in content.");
	let end = start + secret.len();
	let write_policy = serde_json::from_value(serde_json::json!({
		"exclusions": [{"start": start, "end": end}],
	}))
	.expect("Failed to build write_policy.");
	let put = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs write_policy sample".to_string()),
			write_policy: Some(write_policy),
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: content.to_string(),
		})
		.await
		.expect("Failed to put doc with write policy.");
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(&service.db.pool, put.doc_id, std::time::Duration::from_secs(15))
			.await,
		"Expected doc outbox to reach DONE."
	);

	let chunk_id = fetch_first_doc_chunk_id(&service, put.doc_id)
		.await
		.expect("Expected chunk id from transformed doc.");
	let excerpt = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: put.doc_id,
			level: "L1".to_string(),
			chunk_id: Some(chunk_id),
			quote: None,
			position: None,
			explain: None,
		})
		.await
		.expect("Failed to hydrate excerpt by chunk_id.");

	assert!(excerpt.verification.verified);
	assert!(!excerpt.excerpt.is_empty());
	assert!(!excerpt.excerpt.contains(secret));
	assert_eq!(excerpt.verification.content_hash, put.content_hash);
	assert!(put.write_policy_audit.is_some());

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_respects_doc_ts_filter() {
	let Some(ctx) = setup_docs_context().await else { return };
	let (
		test_db,
		service,
		shared_knowledge_doc,
		older_shared_knowledge_doc,
		private_chat_doc,
		handle,
		shutdown,
	) = create_docs_search_filter_fixture(ctx).await;
	let doc_ts_windowed_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: Some("project_shared".to_string()),
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: Some("2026-01-01T00:00:00Z".to_string()),
			ts_lte: Some("2026-12-31T23:59:59Z".to_string()),
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs by doc_ts range.");
	let doc_ts_windowed_ids =
		doc_ts_windowed_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(doc_ts_windowed_ids.contains(&shared_knowledge_doc));
	assert!(!doc_ts_windowed_ids.contains(&older_shared_knowledge_doc));
	assert!(!doc_ts_windowed_ids.contains(&private_chat_doc));

	cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_sparse_mode_records_expected_vector_search_channels() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let doc = put_test_doc(&service).await;
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(&service.db.pool, doc.doc_id, std::time::Duration::from_secs(15))
			.await,
		"Expected doc outbox to reach DONE."
	);

	let cases = [
		("off", vec!["dense"]),
		("on", vec!["dense", "sparse"]),
		("auto", vec!["dense", "sparse"]),
	];

	for (sparse_mode, expected_channels) in cases {
		let response = service
			.docs_search_l0(DocsSearchL0Request {
				tenant_id: "t".to_string(),
				project_id: "p".to_string(),
				caller_agent_id: "reader".to_string(),
				scope: None,
				status: None,
				doc_type: None,
				sparse_mode: Some(sparse_mode.to_string()),
				domain: None,
				repo: None,
				agent_id: None,
				thread_id: None,
				updated_after: None,
				updated_before: None,
				ts_gte: None,
				ts_lte: None,
				read_profile: "private_plus_project".to_string(),
				query: "https://elf.example/docs?query=peregrine".to_string(),
				top_k: Some(20),
				candidate_k: Some(50),
				explain: Some(true),
			})
			.await
			.expect("Failed to search docs with sparse_mode set.");
		let trajectory = response.trajectory.as_ref().expect("Expected explain trajectory.");
		let vector_search_stats = trajectory_stage_stats(trajectory, "vector_search")
			.expect("Expected vector_search stage in trajectory.");
		let vector_search_channels = vector_search_stats
			.get("channels")
			.and_then(serde_json::Value::as_array)
			.expect("Expected vector_search stats channels.");
		let observed_channels = vector_search_channels
			.iter()
			.map(|channel| channel.as_str().expect("Expected channel string.").to_string())
			.collect::<Vec<_>>();

		assert_eq!(observed_channels, expected_channels);
	}

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_filters_include_and_exclude_by_doc_type_and_domain_or_repo() {
	let Some(ctx) = setup_docs_context().await else { return };
	let docs = seed_docs_filter_fixtures(&ctx).await;
	let DocsContext { test_db, service } = ctx;
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	for doc_id in [
		docs.search_domain_doc_id,
		docs.search_other_domain_doc_id,
		docs.repo_doc_id,
		docs.repo_other_doc_id,
	]
	.iter()
	{
		assert!(
			wait_for_doc_outbox_done(&service.db.pool, *doc_id, std::time::Duration::from_secs(15))
				.await,
			"Expected docs outbox to reach DONE."
		);
	}

	let search_domain_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: Some("project_shared".to_string()),
			status: None,
			doc_type: Some("search".to_string()),
			sparse_mode: None,
			domain: Some("docs.example.com".to_string()),
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs by domain.");
	let search_domain_result_ids =
		search_domain_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(search_domain_result_ids.contains(&docs.search_domain_doc_id));
	assert!(!search_domain_result_ids.contains(&docs.search_other_domain_doc_id));
	assert!(!search_domain_result_ids.contains(&docs.repo_doc_id));
	assert!(!search_domain_result_ids.contains(&docs.repo_other_doc_id));

	let repo_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: Some("project_shared".to_string()),
			status: None,
			doc_type: Some("dev".to_string()),
			sparse_mode: None,
			domain: None,
			repo: Some("elf-org/docs".to_string()),
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs by repo.");
	let repo_result_ids =
		repo_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(repo_result_ids.contains(&docs.repo_doc_id));
	assert!(!repo_result_ids.contains(&docs.repo_other_doc_id));
	assert!(!repo_result_ids.contains(&docs.search_domain_doc_id));
	assert!(!repo_result_ids.contains(&docs.search_other_domain_doc_id));

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn seed_docs_filter_fixtures(ctx: &DocsContext) -> DocsFilterFixtureIds {
	let search_domain_doc = put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("search"),
		"Docs domain include sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "search",
			"ts": "2026-02-25T12:00:00Z",
			"query": "How to fetch docs",
			"domain": "docs.example.com",
			"url": "https://docs.example.com/guide",
		}),
		TEST_CONTENT,
	)
	.await;
	let search_other_domain_doc = put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("search"),
		"Docs domain exclude sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "search",
			"ts": "2026-02-25T12:00:00Z",
			"query": "How to build",
			"domain": "api.example.org",
			"url": "https://api.example.org/reference",
		}),
		TEST_CONTENT,
	)
	.await;
	let repo_doc = put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("dev"),
		"Docs repo include sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "dev",
			"ts": "2026-02-25T12:00:00Z",
			"repo": "elf-org/docs",
			"commit_sha": "9f0a3f4c4eb58bfcf4a5f4f9d0c7be0e13c2f8d19",
		}),
		TEST_CONTENT,
	)
	.await;
	let repo_other_doc = put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("dev"),
		"Docs repo exclude sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "dev",
			"ts": "2026-02-25T12:00:00Z",
			"repo": "other-org/docs",
			"commit_sha": "4e3d9ec4d2a59a2f6c7d7f3d4c6e8a5b1f7b9d3f",
		}),
		TEST_CONTENT,
	)
	.await;

	DocsFilterFixtureIds {
		search_domain_doc_id: search_domain_doc.doc_id,
		search_other_domain_doc_id: search_other_domain_doc.doc_id,
		repo_doc_id: repo_doc.doc_id,
		repo_other_doc_id: repo_other_doc.doc_id,
	}
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_recency_bias_orders_newer_doc_first_and_records_projection_signals() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, mut service } = ctx;

	configure_recency_bias_settings(&mut service);

	let (handle, shutdown) = seed_recency_bias_docs_for_search(&service).await;

	assert_docs_search_l0_recency_projection(&service).await;

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn seed_recency_bias_docs_for_search(service: &ElfService) -> (JoinHandle<()>, Sender<()>) {
	let newer_doc = put_test_doc_with(
		service,
		"owner",
		"project_shared",
		Some("knowledge"),
		"Recency newer doc",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-27T12:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let older_doc = put_test_doc_with(
		service,
		"owner",
		"project_shared",
		Some("knowledge"),
		"Recency older doc",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-20T12:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let (handle, shutdown) = spawn_doc_worker(service).await;

	assert!(
		wait_for_doc_outbox_done(
			&service.db.pool,
			newer_doc.doc_id,
			std::time::Duration::from_secs(15),
		)
		.await,
		"Expected newer doc outbox to reach DONE."
	);
	assert!(
		wait_for_doc_outbox_done(
			&service.db.pool,
			older_doc.doc_id,
			std::time::Duration::from_secs(15),
		)
		.await,
		"Expected older doc outbox to reach DONE."
	);

	let older_ts = OffsetDateTime::parse("2020-01-01T00:00:00Z", &Rfc3339)
		.expect("Failed to parse older doc timestamp.");

	sqlx::query("UPDATE doc_documents SET updated_at = $1 WHERE doc_id = $2")
		.bind(older_ts)
		.bind(older_doc.doc_id)
		.execute(&service.db.pool)
		.await
		.expect("Failed to set deterministic updated_at for older doc.");

	(handle, shutdown)
}

async fn assert_docs_search_l0_recency_projection(service: &ElfService) {
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
			top_k: Some(2),
			candidate_k: Some(20),
			explain: Some(true),
		})
		.await
		.expect("Failed to search docs for recency ordering.");
	let ordered_ids = results.items.iter().map(|item| item.doc_id).collect::<Vec<_>>();

	assert!(ordered_ids.len() >= 2);

	let newest_id = results
		.items
		.iter()
		.max_by_key(|item| item.updated_at.unix_timestamp())
		.expect("Expected returned item.")
		.doc_id;

	assert_eq!(results.items[0].doc_id, newest_id);
	assert!(results.items[0].updated_at > results.items[1].updated_at);

	let trajectory = results.trajectory.as_ref().expect("Expected explain trajectory.");
	let result_projection = trajectory_stage_stats(trajectory, "result_projection")
		.expect("Expected result_projection stage in trajectory.");

	assert!(result_projection.get("pre_authorization_candidates").is_some());
	assert!(result_projection.get("returned_items").is_some());
	assert!(result_projection.get("recency_tau_days").is_some());
	assert!(result_projection.get("tie_breaker_weight").is_some());
	assert_eq!(
		result_projection.get("recency_boost_applied"),
		Some(&serde_json::Value::Bool(true))
	);
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
			write_policy: None,
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
			write_policy: None,
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
			write_policy: None,
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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_returns_pointer_and_explain_trajectory() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let doc = put_test_doc(&service).await;
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(&service.db.pool, doc.doc_id, std::time::Duration::from_secs(15))
			.await,
		"Expected doc outbox to reach DONE."
	);

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
			explain: Some(true),
		})
		.await
		.expect("Failed to search docs.");

	assert_eq!(
		results.trajectory.as_ref().map(|trajectory| trajectory.schema.as_str()),
		Some("doc_retrieval_trajectory/v1")
	);
	assert!(results.trajectory.is_some());
	assert!(!results.items.is_empty());
	assert!(results.items[0].pointer.schema == "source_ref/v1");
	assert!(!results.items[0].pointer.reference.doc_id.is_nil());
	assert!(!results.items[0].pointer.reference.chunk_id.is_nil());
	assert_eq!(results.items[0].pointer.resolver, "elf_doc_ext/v1");
	assert!(!results.trace_id.is_nil());

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_note_pointer_roundtrip_hydrates_doc() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let doc = put_test_doc(&service).await;
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(&service.db.pool, doc.doc_id, std::time::Duration::from_secs(15))
			.await,
		"Expected doc outbox to reach DONE."
	);

	let (source_ref, source_ref_doc_id, source_ref_chunk_id) =
		fetch_docs_pointer_source_ref(&service).await;
	let note_id = add_note_with_pointer_source_ref(&service, source_ref.clone()).await;

	assert!(
		wait_for_note_outbox_done(&service.db.pool, note_id, std::time::Duration::from_secs(15))
			.await,
		"Expected note outbox to reach DONE."
	);

	let search_results = service
		.search_raw_quick(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "agent".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "peregrine".to_string(),
			top_k: Some(5),
			candidate_k: Some(20),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Failed to search note with doc pointer source_ref.");
	let has_pointer_source_ref =
		search_results.items.into_iter().any(|item| item.source_ref == source_ref);

	assert!(
		has_pointer_source_ref,
		"Expected search result to include note with pointer source_ref."
	);

	let excerpt = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: source_ref_doc_id,
			level: "L1".to_string(),
			chunk_id: Some(source_ref_chunk_id),
			quote: None,
			position: None,
			explain: None,
		})
		.await
		.expect("Failed to hydrate excerpt from pointer source_ref.");

	assert!(excerpt.verification.verified);

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn fetch_docs_pointer_source_ref(service: &ElfService) -> (serde_json::Value, Uuid, Uuid) {
	let search = service
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
		.expect("Failed to search docs for source_ref pointer.");

	assert!(!search.items.is_empty(), "Expected docs_search_l0 to return source_ref pointer.");

	let pointer = search.items[0].pointer.clone();
	let source_ref =
		serde_json::to_value(&pointer).expect("Failed to serialize docs_search_l0 pointer.");

	(source_ref, pointer.reference.doc_id, pointer.reference.chunk_id)
}

async fn add_note_with_pointer_source_ref(
	service: &ElfService,
	source_ref: serde_json::Value,
) -> Uuid {
	let note = service
		.add_note(AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "agent".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("doc_pointer_note".to_string()),
				text: "Peregrine note for source_ref hydration check.".to_string(),
				structured: None,
				importance: 0.5,
				confidence: 0.9,
				ttl_days: None,
				source_ref,
				write_policy: None,
			}],
		})
		.await
		.expect("Failed to add note from docs pointer.");

	note.results[0].note_id.expect("Expected note_id in add_note result.")
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_excerpts_get_supports_l0_and_returns_locator_and_optional_trajectory() {
	let Some(ctx) = setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let doc = put_test_doc(&service).await;
	let (handle, shutdown) = spawn_doc_worker(&service).await;

	assert!(
		wait_for_doc_outbox_done(&service.db.pool, doc.doc_id, std::time::Duration::from_secs(15))
			.await,
		"Expected doc outbox to reach DONE."
	);

	let excerpt = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: doc.doc_id,
			level: "L0".to_string(),
			chunk_id: None,
			quote: Some(TextQuoteSelector {
				exact: "Keyword: peregrine.".to_string(),
				prefix: Some("evidence. ".to_string()),
				suffix: Some("\nSecond".to_string()),
			}),
			position: None,
			explain: Some(true),
		})
		.await
		.expect("Failed to hydrate excerpt.");

	assert_eq!(excerpt.locator.selector_kind, "quote");
	assert!(excerpt.locator.match_end_offset > excerpt.locator.match_start_offset);
	assert!(excerpt.excerpt.len() <= 256);
	assert!(excerpt.trajectory.is_some());
	assert_eq!(
		excerpt.trajectory.as_ref().map(|trajectory| trajectory.schema.as_str()),
		Some("doc_retrieval_trajectory/v1")
	);
	assert!(!excerpt.trace_id.is_nil());

	let no_explain = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: doc.doc_id,
			level: "L0".to_string(),
			chunk_id: None,
			quote: Some(TextQuoteSelector {
				exact: "Keyword: peregrine.".to_string(),
				prefix: Some("evidence. ".to_string()),
				suffix: Some("\nSecond".to_string()),
			}),
			position: None,
			explain: Some(false),
		})
		.await
		.expect("Failed to hydrate excerpt.");

	assert!(no_explain.trajectory.is_none());

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
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
