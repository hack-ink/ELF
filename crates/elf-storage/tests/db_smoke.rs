#[tokio::test]
async fn db_connects_and_bootstraps() {
    let dsn = match std::env::var("ELF_TEST_PG_DSN") {
        Ok(value) => value,
        Err(_) => return,
    };
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
