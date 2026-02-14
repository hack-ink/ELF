use uuid::Uuid;

use elf_config::Postgres;
use elf_storage::{db::Db, outbox};
use elf_testkit::TestDatabase;

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn enqueues_outbox_job() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping enqueues_outbox_job; set ELF_PG_DSN to run this test.");

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	outbox::enqueue_outbox(&db.pool, Uuid::new_v4(), "UPSERT", "test:vector:1")
		.await
		.expect("Failed to enqueue outbox.");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
