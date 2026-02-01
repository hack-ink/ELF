#[tokio::test]
async fn enqueues_outbox_job() {
    let dsn = std::env::var("ELF_TEST_PG_DSN")
        .expect("ELF_TEST_PG_DSN must be set for outbox test.");
    let cfg = elf_config::Postgres {
        dsn,
        pool_max_conns: 1,
    };
    let db = elf_storage::db::Db::connect(&cfg)
        .await
        .expect("Failed to connect to Postgres.");
    db.ensure_schema(1536)
        .await
        .expect("Failed to ensure schema.");

    elf_storage::outbox::enqueue_outbox(
        &db,
        uuid::Uuid::new_v4(),
        "UPSERT",
        "test:vector:1",
    )
    .await
    .expect("Failed to enqueue outbox.");
}
