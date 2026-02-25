use std::sync::{Arc, atomic::AtomicUsize};

use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
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

async fn insert_note(
	pool: &PgPool,
	note_id: Uuid,
	now: OffsetDateTime,
	embedding_version: &str,
	text: &str,
) {
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
	.bind(text)
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
	.execute(pool)
	.await
	.expect("Failed to insert memory note.");
}

async fn insert_embedding(pool: &PgPool, note_id: Uuid, embedding_version: &str, vec_text: &str) {
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
	.bind(4_096_i32)
	.bind(vec_text)
	.execute(pool)
	.await
	.expect("Failed to insert embedding.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn active_notes_have_vectors() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping active_notes_have_vectors; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping active_notes_have_vectors; set ELF_QDRANT_URL to run this test.");

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
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let note_id = Uuid::new_v4();
	let now = OffsetDateTime::now_utc();
	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);
	let vec_text = build_zero_vector_text(4_096);

	insert_note(
		&service.db.pool,
		note_id,
		now,
		embedding_version.as_str(),
		"Fact: Vector row exists.",
	)
	.await;
	insert_embedding(&service.db.pool, note_id, embedding_version.as_str(), vec_text.as_str())
		.await;

	let missing: i64 = sqlx::query_scalar(
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
	.fetch_one(&service.db.pool)
	.await
	.expect("Failed to query missing embeddings.");

	assert_eq!(missing, 0);

	let dim: i32 = sqlx::query_scalar(
		"SELECT embedding_dim FROM note_embeddings WHERE note_id = $1 AND embedding_version = $2",
	)
	.bind(note_id)
	.bind(embedding_version.as_str())
	.fetch_one(&service.db.pool)
	.await
	.expect("Failed to query embedding dim.");

	assert_eq!(dim, 4_096);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
