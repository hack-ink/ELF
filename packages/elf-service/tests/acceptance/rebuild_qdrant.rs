use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use time::OffsetDateTime;
use uuid::Uuid;

use super::{SpyEmbedding, SpyExtractor, StubRerank};
use elf_service::Providers;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rebuild_uses_postgres_vectors_only() {
	let Some(test_db) = super::test_db().await else {
		eprintln!("Skipping rebuild_uses_postgres_vectors_only; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = super::test_qdrant_url() else {
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
		Arc::new(SpyEmbedding { vector_dim: 3, calls: embed_calls.clone() }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg = super::test_config(test_db.dsn().to_string(), qdrant_url, 3, collection);
	let service = super::build_service(cfg, providers).await.expect("Failed to build service.");

	super::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	super::reset_qdrant_collection(
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

	sqlx::query!(
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
		note_id,
		"t",
		"p",
		"a",
		"agent_private",
		"fact",
		None::<String>,
		"Fact: Rebuild works.",
		0.5_f32,
		0.9_f32,
		"active",
		now,
		now,
		None::<OffsetDateTime>,
		embedding_version.as_str(),
		serde_json::json!({}),
		0_i64,
		None::<OffsetDateTime>,
	)
	.execute(&service.db.pool)
	.await
	.expect("Failed to insert memory note.");

	let chunk_id = Uuid::new_v4();
	let text = "Fact: Rebuild works.";

	sqlx::query!(
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
		chunk_id,
		note_id,
		0_i32,
		0_i32,
		text.len() as i32,
		text,
		embedding_version.as_str(),
	)
	.execute(&service.db.pool)
	.await
	.expect("Failed to insert chunk metadata.");

	sqlx::query!(
		"\
		INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
		VALUES ($1, $2, $3, $4::text::vector)",
		chunk_id,
		embedding_version.as_str(),
		3_i32,
		"[0,0,0]",
	)
	.execute(&service.db.pool)
	.await
	.expect("Failed to insert chunk embedding.");

	let report = service.rebuild_qdrant().await.expect("Rebuild failed.");

	assert_eq!(report.missing_vector_count, 0);

	assert!(report.rebuilt_count >= 1);

	assert_eq!(embed_calls.load(Ordering::SeqCst), 0);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
