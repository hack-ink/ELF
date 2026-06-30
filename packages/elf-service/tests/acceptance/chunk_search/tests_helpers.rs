use std::{
	collections::HashMap,
	sync::{Arc, atomic::AtomicUsize},
};

use qdrant_client::{
	Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::{self, SpyExtractor, StubEmbedding};
use elf_config::ProviderConfig;
use elf_service::{BoxFuture, ElfService, Providers, RerankProvider, Result};
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};
use elf_testkit::TestDatabase;

pub(super) struct TestContext {
	pub(super) service: ElfService,
	pub(super) test_db: TestDatabase,
	pub(super) embedding_version: String,
}

pub(super) struct KeywordRerank {
	pub(super) keyword: &'static str,
}
impl RerankProvider for KeywordRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
		let keyword = self.keyword;

		Box::pin(async move {
			Ok(docs.iter().map(|doc| if doc.contains(keyword) { 1.0 } else { 0.1 }).collect())
		})
	}
}

pub(super) fn build_providers<R>(rerank: R) -> Providers
where
	R: RerankProvider + Send + Sync + 'static,
{
	Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(rerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	)
}

pub(super) fn build_payload(
	note_id: Uuid,
	chunk_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
) -> Payload {
	let mut payload = Payload::new();

	payload.insert("note_id", note_id.to_string());
	payload.insert("chunk_id", chunk_id.to_string());
	payload.insert("chunk_index", Value::from(chunk_index));
	payload.insert("start_offset", Value::from(start_offset));
	payload.insert("end_offset", Value::from(end_offset));
	payload.insert("tenant_id", "t");
	payload.insert("project_id", "p");
	payload.insert("agent_id", "a");
	payload.insert("scope", "agent_private");
	payload.insert("status", "active");

	payload
}

pub(super) fn build_vectors(text: &str) -> HashMap<String, Vector> {
	let mut vectors = HashMap::new();

	vectors.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec![0.0_f32; 4_096]));
	vectors.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(text.to_string(), BM25_MODEL)),
	);

	vectors
}

pub(super) async fn setup_context(test_name: &str, providers: Providers) -> Option<TestContext> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

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
	let service =
		acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	reset_collection(&service).await;

	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);

	Some(TestContext { service, test_db, embedding_version })
}

pub(super) async fn reset_collection(service: &ElfService) {
	acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.qdrant.collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant collection.");
}

pub(super) async fn insert_note<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	insert_note_with_importance_and_source_ref(
		executor,
		note_id,
		note_text,
		embedding_version,
		0.4_f32,
		0.9_f32,
		"agent_private",
		serde_json::json!({}),
	)
	.await;
}

pub(super) async fn insert_note_with_importance<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
	importance: f32,
	confidence: f32,
	scope: &str,
) where
	E: PgExecutor<'e>,
{
	insert_note_with_importance_and_source_ref(
		executor,
		note_id,
		note_text,
		embedding_version,
		importance,
		confidence,
		scope,
		serde_json::json!({}),
	)
	.await;
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_note_with_importance_and_source_ref<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
	importance: f32,
	confidence: f32,
	scope: &str,
	source_ref: Value,
) where
	E: PgExecutor<'e>,
{
	let now = OffsetDateTime::now_utc();

	sqlx::query(
		"\
INSERT INTO memory_notes (
	note_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref,
	hit_count,
	last_hit_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15,
	$16,
	$17,
	$18
)",
	)
	.bind(note_id)
	.bind("t")
	.bind("p")
	.bind("a")
	.bind(scope)
	.bind("fact")
	.bind(Option::<String>::None)
	.bind(note_text)
	.bind(importance)
	.bind(confidence)
	.bind("active")
	.bind(now)
	.bind(now)
	.bind(Option::<OffsetDateTime>::None)
	.bind(embedding_version)
	.bind(source_ref)
	.bind(0_i64)
	.bind(Option::<OffsetDateTime>::None)
	.execute(executor)
	.await
	.expect("Failed to insert memory note.");
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_summary_field_row<'e, E>(
	executor: E,
	field_id: Uuid,
	note_id: Uuid,
	summary: &str,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO memory_note_fields (field_id, note_id, field_kind, item_index, text)
VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(field_id)
	.bind(note_id)
	.bind("summary")
	.bind(0_i32)
	.bind(summary)
	.execute(executor)
	.await
	.expect("Failed to insert note summary field.");
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_chunk<'e, E>(
	executor: E,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO memory_note_chunks (
	chunk_id,
	note_id,
	chunk_index,
	start_offset,
	end_offset,
	text,
	embedding_version
)
VALUES ($1, $2, $3, $4, $5, $6, $7)",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(chunk_index)
	.bind(start_offset)
	.bind(end_offset)
	.bind(text)
	.bind(embedding_version)
	.execute(executor)
	.await
	.expect("Failed to insert chunk metadata.");
}

pub(super) async fn upsert_point(
	service: &ElfService,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
) {
	let payload = build_payload(note_id, chunk_id, chunk_index, start_offset, end_offset);
	let vectors = build_vectors(text);
	let point = PointStruct::new(chunk_id.to_string(), vectors, payload);

	service
		.qdrant
		.client
		.upsert_points(
			UpsertPointsBuilder::new(service.qdrant.collection.clone(), vec![point]).wait(true),
		)
		.await
		.expect("Failed to upsert Qdrant point.");
}
