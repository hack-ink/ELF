#[tokio::test]
async fn active_notes_have_vectors() {
    let dsn = match std::env::var("ELF_TEST_PG_DSN") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Skipping active_notes_have_vectors; set ELF_TEST_PG_DSN to run this test.");
            return;
        }
    };
    let qdrant_url = match std::env::var("ELF_TEST_QDRANT_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Skipping active_notes_have_vectors; set ELF_TEST_QDRANT_URL to run this test.");
            return;
        }
    };

    let cfg = super::test_config(dsn, qdrant_url, 3);
    let providers = elf_service::Providers::new(
        std::sync::Arc::new(super::StubEmbedding { vector_dim: 3 }),
        std::sync::Arc::new(super::StubRerank),
        std::sync::Arc::new(super::SpyExtractor {
            calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            payload: serde_json::json!({ "notes": [] }),
        }),
    );
    let service = super::build_service(cfg, providers)
        .await
        .expect("Failed to build service.");

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
    .bind("Fact: Vector row exists.")
    .bind(0.4_f32)
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

    let missing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memory_notes n \
         LEFT JOIN note_embeddings e ON n.note_id = e.note_id AND n.embedding_version = e.embedding_version \
         WHERE n.status = 'active' AND e.note_id IS NULL",
    )
    .fetch_one(&service.db.pool)
    .await
    .expect("Failed to query missing embeddings.");
    assert_eq!(missing, 0);

    let dim: i32 = sqlx::query_scalar(
        "SELECT embedding_dim FROM note_embeddings WHERE note_id = $1 AND embedding_version = $2",
    )
    .bind(note_id)
    .bind(&embedding_version)
    .fetch_one(&service.db.pool)
    .await
    .expect("Failed to query embedding dim.");
    assert_eq!(dim, 3);
}
