use sqlx::Connection;

const TEST_DB_LOCK_KEY: i64 = 0x454C4601;

struct DbLock {
    _conn: sqlx::PgConnection,
}

async fn acquire_db_lock(dsn: &str) -> DbLock {
    let mut conn = sqlx::PgConnection::connect(dsn)
        .await
        .expect("Failed to connect for DB lock.");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(TEST_DB_LOCK_KEY)
        .execute(&mut conn)
        .await
        .expect("Failed to acquire DB lock.");
    DbLock { _conn: conn }
}

#[tokio::test]
async fn enqueues_outbox_job() {
    let dsn = std::env::var("ELF_PG_DSN").expect("ELF_PG_DSN must be set for outbox test.");
	let _lock = acquire_db_lock(&dsn).await;
    let cfg = elf_config::Postgres {
        dsn,
        pool_max_conns: 1,
    };
    let db = elf_storage::db::Db::connect(&cfg)
        .await
        .expect("Failed to connect to Postgres.");
    db.ensure_schema(3)
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
