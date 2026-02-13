use std::sync::{Arc, atomic::AtomicUsize};

use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{SpyExtractor, StubEmbedding, StubRerank};
use elf_service::{ElfService, Providers};

const VECTOR_DIM: i32 = 4_096;

fn build_providers() -> Providers {
	Providers::new(
		Arc::new(StubEmbedding { vector_dim: VECTOR_DIM as usize }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	)
}

fn embedding_version(service: &ElfService) -> String {
	format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	)
}

fn zero_vector_text(vector_dim: usize) -> String {
	let mut buf = String::with_capacity(2 + (vector_dim * 2));

	buf.push('[');
	for i in 0..vector_dim {
		if i > 0 {
			buf.push(',');
		}

		buf.push('0');
	}
	buf.push(']');

	buf
}

async fn setup_context(test_name: &str) -> Option<(elf_testkit::TestDatabase, ElfService)> {
	let Some(test_db) = super::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = super::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let cfg =
		super::test_config(test_db.dsn().to_string(), qdrant_url, VECTOR_DIM as usize, collection);
	let service =
		super::build_service(cfg, build_providers()).await.expect("Failed to build service.");

	super::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	Some((test_db, service))
}

async fn insert_active_note<'e, E>(executor: E, note_id: Uuid, embedding_version: &str)
where
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
	.bind("Fact: Vector row exists.")
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

async fn insert_embedding<'e, E>(
	executor: E,
	note_id: Uuid,
	embedding_version: &str,
	vector_dim: i32,
) where
	E: PgExecutor<'e>,
{
	let vec_text = zero_vector_text(vector_dim as usize);

	sqlx::query(
		"\
INSERT INTO note_embeddings (
	note_id,
	embedding_version,
	embedding_dim,
	vec
)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(note_id)
	.bind(embedding_version)
	.bind(vector_dim)
	.bind(vec_text.as_str())
	.execute(executor)
	.await
	.expect("Failed to insert embedding.");
}

async fn count_missing_embeddings<'e, E>(executor: E, note_id: Uuid) -> i64
where
	E: PgExecutor<'e>,
{
	sqlx::query_scalar(
		"\
SELECT COUNT(*) AS \"missing!\"
FROM memory_notes n
LEFT JOIN note_embeddings e
ON n.note_id = e.note_id
AND n.embedding_version = e.embedding_version
WHERE n.note_id = $1
		AND e.note_id IS NULL",
	)
	.bind(note_id)
	.fetch_one(executor)
	.await
	.expect("Failed to query missing embeddings.")
}

async fn embedding_dim<'e, E>(executor: E, note_id: Uuid, embedding_version: &str) -> i32
where
	E: PgExecutor<'e>,
{
	sqlx::query_scalar(
		"SELECT embedding_dim FROM note_embeddings WHERE note_id = $1 AND embedding_version = $2",
	)
	.bind(note_id)
	.bind(embedding_version)
	.fetch_one(executor)
	.await
	.expect("Failed to query embedding dim.")
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn active_notes_have_vectors() {
	let Some((test_db, service)) = setup_context("active_notes_have_vectors").await else {
		return;
	};
	let note_id = Uuid::new_v4();
	let embedding_version = embedding_version(&service);

	insert_active_note(&service.db.pool, note_id, &embedding_version).await;
	insert_embedding(&service.db.pool, note_id, &embedding_version, VECTOR_DIM).await;

	let missing = count_missing_embeddings(&service.db.pool, note_id).await;

	assert_eq!(missing, 0);

	let dim = embedding_dim(&service.db.pool, note_id, &embedding_version).await;

	assert_eq!(dim, VECTOR_DIM);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
