use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder};

use super::{SpyEmbedding, SpyExtractor, StubRerank, build_service, test_config, test_dsn, test_qdrant_url};

#[tokio::test]
async fn rebuild_uses_postgres_vectors_only() {
    let Some(dsn) = test_dsn() else {
        eprintln!("Skipping rebuild_uses_postgres_vectors_only; set ELF_PG_DSN to run this test.");
        return;
    };
    let Some(qdrant_url) = test_qdrant_url() else {
        eprintln!("Skipping rebuild_uses_postgres_vectors_only; set ELF_QDRANT_URL to run this test.");
        return;
    };
	let _guard = super::test_lock(&dsn)
		.await
		.expect("Failed to acquire test lock.");
    let embed_calls = Arc::new(AtomicUsize::new(0));
    let extractor = SpyExtractor {
        calls: Arc::new(AtomicUsize::new(0)),
        payload: serde_json::json!({ "notes": [] }),
    };
    let providers = elf_service::Providers::new(
        Arc::new(SpyEmbedding {
            vector_dim: 3,
            calls: embed_calls.clone(),
        }),
        Arc::new(StubRerank),
        Arc::new(extractor),
    );

    let cfg = test_config(dsn, qdrant_url, 3);
    let service = build_service(cfg, providers)
        .await
        .expect("Failed to build service.");
	super::reset_db(&service.db.pool)
		.await
		.expect("Failed to reset test database.");

    let _ = service
        .qdrant
        .client
        .delete_collection(service.qdrant.collection.clone())
        .await;
    service
        .qdrant
        .client
        .create_collection(
            CreateCollectionBuilder::new(service.qdrant.collection.clone())
                .vectors_config(VectorParamsBuilder::new(3, Distance::Cosine)),
        )
        .await
        .expect("Failed to create Qdrant collection.");

    let note_id = uuid::Uuid::new_v4();
    let now = time::OffsetDateTime::now_utc();
    let embedding_version = format!(
        "{}:{}:{}",
        service.cfg.providers.embedding.provider_id,
        service.cfg.providers.embedding.model,
        service.cfg.storage.qdrant.vector_dim
    );

    sqlx::query(
        "INSERT INTO memory_notes \
         (note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
    )
    .bind(note_id)
    .bind("t")
    .bind("p")
    .bind("a")
    .bind("agent_private")
    .bind("fact")
    .bind::<Option<String>>(None)
    .bind("Fact: Rebuild works.")
    .bind(0.5_f32)
    .bind(0.9_f32)
    .bind("active")
    .bind(now)
    .bind(now)
    .bind::<Option<time::OffsetDateTime>>(None)
    .bind(&embedding_version)
    .bind(serde_json::json!({}))
    .bind(0_i64)
    .bind::<Option<time::OffsetDateTime>>(None)
    .execute(&service.db.pool)
    .await
    .expect("Failed to insert memory note.");

    sqlx::query(
        "INSERT INTO note_embeddings (note_id, embedding_version, embedding_dim, vec) \
         VALUES ($1,$2,$3,$4::vector)",
    )
    .bind(note_id)
    .bind(&embedding_version)
    .bind(3_i32)
    .bind("[0,0,0]")
    .execute(&service.db.pool)
    .await
    .expect("Failed to insert embedding.");

    let report = service
        .rebuild_qdrant()
        .await
        .expect("Rebuild failed.");
    assert_eq!(report.missing_vector_count, 0);
    assert!(report.rebuilt_count >= 1);
    assert_eq!(embed_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
