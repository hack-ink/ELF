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

pub(crate) struct TestContext {
	pub(crate) service: ElfService,
	pub(crate) test_db: TestDatabase,
	pub(crate) embedding_version: String,
}

pub(crate) struct UpsertPointArgs<'a> {
	pub(crate) chunk_id: Uuid,
	pub(crate) note_id: Uuid,
	pub(crate) chunk_index: i32,
	pub(crate) start_offset: i32,
	pub(crate) end_offset: i32,
	pub(crate) text: &'a str,
	pub(crate) dense: Vec<f32>,
}

struct KeywordRerank {
	keyword: &'static str,
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

pub(crate) async fn setup_context(test_name: &str) -> Option<TestContext> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(KeywordRerank { keyword: "ZEBRA" }),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
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
	acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.qdrant.collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant collection.");

	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);

	Some(TestContext { service, test_db, embedding_version })
}

pub(crate) async fn insert_note<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
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
	.bind("agent_private")
	.bind("fact")
	.bind(Option::<String>::None)
	.bind(note_text)
	.bind(0.4_f32)
	.bind(0.9_f32)
	.bind("active")
	.bind(now)
	.bind(now)
	.bind(Option::<OffsetDateTime>::None)
	.bind(embedding_version)
	.bind(serde_json::json!({}))
	.bind(0_i64)
	.bind(Option::<OffsetDateTime>::None)
	.execute(executor)
	.await
	.expect("Failed to insert memory note.");
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_chunk<'e, E>(
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

pub(crate) async fn insert_chunk_embedding<'e, E>(
	executor: E,
	chunk_id: Uuid,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	let vec_text = vec_text_zeros();

	sqlx::query(
		"\
INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(chunk_id)
	.bind(embedding_version)
	.bind(4_096_i32)
	.bind(vec_text.as_str())
	.execute(executor)
	.await
	.expect("Failed to insert chunk embedding.");
}

pub(crate) async fn insert_fact_field_row<'e, E>(
	executor: E,
	field_id: Uuid,
	note_id: Uuid,
	fact_text: &str,
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
	.bind("fact")
	.bind(0_i32)
	.bind(fact_text)
	.execute(executor)
	.await
	.expect("Failed to insert note field.");
}

pub(crate) async fn insert_fact_field_embedding<'e, E>(
	executor: E,
	field_id: Uuid,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	let vec_text = vec_text_zeros();

	sqlx::query(
		"\
INSERT INTO note_field_embeddings (field_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(field_id)
	.bind(embedding_version)
	.bind(4_096_i32)
	.bind(vec_text.as_str())
	.execute(executor)
	.await
	.expect("Failed to insert field embedding.");
}

pub(crate) async fn upsert_point(service: &ElfService, args: UpsertPointArgs<'_>) {
	let payload = build_payload(
		args.note_id,
		args.chunk_id,
		args.chunk_index,
		args.start_offset,
		args.end_offset,
	);
	let vectors = build_vectors(args.text, args.dense);
	let point = PointStruct::new(args.chunk_id.to_string(), vectors, payload);

	service
		.qdrant
		.client
		.upsert_points(
			UpsertPointsBuilder::new(service.qdrant.collection.clone(), vec![point]).wait(true),
		)
		.await
		.expect("Failed to upsert Qdrant point.");
}

fn vec_text_zeros() -> String {
	let mut buf = String::with_capacity(2 + (4_096 * 2));

	buf.push('[');

	for i in 0..4_096 {
		if i > 0 {
			buf.push(',');
		}

		buf.push('0');
	}

	buf.push(']');

	buf
}

fn build_payload(
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

fn build_vectors(text: &str, dense: Vec<f32>) -> HashMap<String, Vector> {
	let mut vectors = HashMap::new();

	vectors.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(dense));
	vectors.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(text.to_string(), BM25_MODEL)),
	);

	vectors
}
