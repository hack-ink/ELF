#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn enqueues_outbox_job() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping enqueues_outbox_job; set ELF_PG_DSN to run this test.");
		return;
	};
	let test_db =
		elf_testkit::TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = elf_config::Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = elf_storage::db::Db::connect(&cfg).await.expect("Failed to connect to Postgres.");
	db.ensure_schema(3).await.expect("Failed to ensure schema.");

	elf_storage::outbox::enqueue_outbox(&db, uuid::Uuid::new_v4(), "UPSERT", "test:vector:1")
		.await
		.expect("Failed to enqueue outbox.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
