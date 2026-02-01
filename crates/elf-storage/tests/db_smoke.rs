#[tokio::test]
async fn db_connects_and_bootstraps() {
    let dsn =
        std::env::var("ELF_PG_DSN").expect("ELF_PG_DSN must be set for db_smoke test.");
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
}
