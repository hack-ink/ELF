use tokio::runtime::Runtime;

use elf_storage::db::Db;
use elf_testkit::TestDatabase;

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn db_connects_and_bootstraps() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping db_connects_and_bootstraps; set ELF_PG_DSN to run this test.");

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = elf_config::Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");
	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
fn chunk_tables_exist_after_bootstrap() {
	let Some(dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping chunk_tables_exist_after_bootstrap; set ELF_PG_DSN to run this test.");

		return;
	};
	let rt = Runtime::new().expect("Failed to build runtime.");
	rt.block_on(async {
		let cfg = elf_config::Postgres { dsn: dsn.clone(), pool_max_conns: 1 };
		let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");
		db.ensure_schema(4_096).await.expect("Failed to ensure schema.");
		let count: i64 = sqlx::query_scalar(
			"SELECT count(*) FROM information_schema.tables WHERE table_name = 'memory_note_chunks'",
		)
		.fetch_one(&db.pool)
		.await
		.expect("Failed to query schema tables.");

		assert_eq!(count, 1);
	});
}
