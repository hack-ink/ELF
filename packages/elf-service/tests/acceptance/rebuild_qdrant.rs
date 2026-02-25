use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::{SpyEmbedding, SpyExtractor, StubRerank};
use elf_service::Providers;

fn build_zero_vector_text(dim: usize) -> String {
	let mut buf = String::with_capacity(2 + (dim * 2));

	buf.push('[');

	for i in 0..dim {
		if i > 0 {
			buf.push(',');
		}

		buf.push('0');
	}

	buf.push(']');

	buf
}

async fn insert_note(pool: &PgPool, note_id: Uuid, now: OffsetDateTime, embedding_version: &str) {
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
	.bind("Fact: Rebuild works.")
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

async fn insert_chunk(pool: &PgPool, chunk_id: Uuid, note_id: Uuid, embedding_version: &str) {
	let text = "Fact: Rebuild works.";

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
	.bind(text.len() as i32)
	.bind(text)
	.bind(embedding_version)
	.execute(pool)
	.await
	.expect("Failed to insert chunk metadata.");
}

async fn insert_chunk_embedding(
	pool: &PgPool,
	chunk_id: Uuid,
	embedding_version: &str,
	vec_text: &str,
) {
	sqlx::query(
		"\
INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(chunk_id)
	.bind(embedding_version)
	.bind(4_096_i32)
	.bind(vec_text)
	.execute(pool)
	.await
	.expect("Failed to insert chunk embedding.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rebuild_uses_postgres_vectors_only() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping rebuild_uses_postgres_vectors_only; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping rebuild_uses_postgres_vectors_only; set ELF_QDRANT_URL to run this test."
		);

		return;
	};
	let embed_calls = Arc::new(AtomicUsize::new(0));
	let extractor = SpyExtractor {
		calls: Arc::new(AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = Providers::new(
		Arc::new(SpyEmbedding { vector_dim: 4_096, calls: embed_calls.clone() }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = crate::acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
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
	.expect("Failed to reset Qdrant collection.");

	let note_id = Uuid::new_v4();
	let now = OffsetDateTime::now_utc();
	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);
	let chunk_id = Uuid::new_v4();
	let vec_text = build_zero_vector_text(4_096);

	insert_note(&service.db.pool, note_id, now, embedding_version.as_str()).await;
	insert_chunk(&service.db.pool, chunk_id, note_id, embedding_version.as_str()).await;
	insert_chunk_embedding(
		&service.db.pool,
		chunk_id,
		embedding_version.as_str(),
		vec_text.as_str(),
	)
	.await;

	let report = service.rebuild_qdrant().await.expect("Rebuild failed.");

	assert_eq!(report.missing_vector_count, 0);
	assert!(report.rebuilt_count >= 1);
	assert_eq!(embed_calls.load(Ordering::SeqCst), 0);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
