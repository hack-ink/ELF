use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use time::OffsetDateTime;
use uuid::Uuid;

use super::{SpyEmbedding, SpyExtractor, StubRerank};
use elf_service::{ElfService, Providers};
use elf_testkit::TestDatabase;

const VECTOR_DIM: u32 = 4_096;
const NOTE_TEXT: &str = "Fact: Rebuild works.";

struct TestContext {
	service: ElfService,
	test_db: TestDatabase,
	embed_calls: Arc<AtomicUsize>,
	embedding_version: String,
}

fn build_zero_vector_text(vector_dim: usize) -> String {
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

async fn setup_context(test_name: &str) -> Option<TestContext> {
	let Some(test_db) = super::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = super::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let embed_calls = Arc::new(AtomicUsize::new(0));
	let extractor = SpyExtractor {
		calls: Arc::new(AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = Providers::new(
		Arc::new(SpyEmbedding { vector_dim: VECTOR_DIM, calls: embed_calls.clone() }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg = super::test_config(test_db.dsn().to_string(), qdrant_url, VECTOR_DIM, collection);
	let service = super::build_service(cfg, providers).await.expect("Failed to build service.");

	super::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	super::reset_qdrant_collection(
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

	Some(TestContext { service, test_db, embed_calls, embedding_version })
}

async fn insert_note(pool: &sqlx::PgPool, note_id: Uuid, embedding_version: &str) {
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
	.bind(NOTE_TEXT)
	.bind(0.5_f32)
	.bind(0.9_f32)
	.bind("active")
	.bind(now)
	.bind(now)
	.bind(Option::<OffsetDateTime>::None)
	.bind(embedding_version)
	.bind(serde_json::json!({}))
	.bind(0_i64)
	.bind(Option::<OffsetDateTime>::None)
	.execute(pool)
	.await
	.expect("Failed to insert memory note.");
}

async fn insert_chunk(pool: &sqlx::PgPool, chunk_id: Uuid, note_id: Uuid, embedding_version: &str) {
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
	.bind(0_i32)
	.bind(0_i32)
	.bind(NOTE_TEXT.len() as i32)
	.bind(NOTE_TEXT)
	.bind(embedding_version)
	.execute(pool)
	.await
	.expect("Failed to insert chunk metadata.");
}

async fn insert_chunk_embedding(pool: &sqlx::PgPool, chunk_id: Uuid, embedding_version: &str) {
	let vec_text = build_zero_vector_text(VECTOR_DIM as usize);

	sqlx::query(
		"\
INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(chunk_id)
	.bind(embedding_version)
	.bind(VECTOR_DIM as i32)
	.bind(vec_text.as_str())
	.execute(pool)
	.await
	.expect("Failed to insert chunk embedding.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rebuild_uses_postgres_vectors_only() {
	let Some(context) = setup_context("rebuild_uses_postgres_vectors_only").await else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();

	insert_note(&context.service.db.pool, note_id, &context.embedding_version).await;
	insert_chunk(&context.service.db.pool, chunk_id, note_id, &context.embedding_version).await;
	insert_chunk_embedding(&context.service.db.pool, chunk_id, &context.embedding_version).await;

	let report = context.service.rebuild_qdrant().await.expect("Rebuild failed.");

	assert_eq!(report.missing_vector_count, 0);
	assert!(report.rebuilt_count >= 1);
	assert_eq!(context.embed_calls.load(Ordering::SeqCst), 0);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
