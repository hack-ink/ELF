#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn db_connects_and_bootstraps() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping db_connects_and_bootstraps; set ELF_PG_DSN to run this test.");
		return;
	};
	let test_db =
		elf_testkit::TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = elf_config::Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = elf_storage::db::Db::connect(&cfg).await.expect("Failed to connect to Postgres.");
	db.ensure_schema(3).await.expect("Failed to ensure schema.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
