use tokio::runtime::Runtime;
use uuid::Uuid;

use elf_config::Postgres;
use elf_storage::db::Db;
use elf_testkit::TestDatabase;

#[test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
fn chunk_tables_exist_after_bootstrap() {
	let Some(dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping chunk_tables_exist_after_bootstrap; set ELF_PG_DSN to run this test.");

		return;
	};
	let rt = Runtime::new().expect("Failed to build runtime.");

	rt.block_on(async {
		let cfg = Postgres { dsn: dsn.clone(), pool_max_conns: 1 };
		let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

		db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

		let count: i64 = sqlx::query_scalar(
			"SELECT count(*) FROM information_schema.tables WHERE table_name = 'memory_note_chunks'",
		)
		.fetch_one(&db.pool)
		.await
		.expect("Failed to query schema tables.");

		assert_eq!(count, 1);

		let count: i64 = sqlx::query_scalar(
			"SELECT count(*) FROM information_schema.tables WHERE table_name = 'memory_ingest_decisions'",
		)
		.fetch_one(&db.pool)
		.await
		.expect("Failed to query schema tables.");

		assert_eq!(count, 1);

		let count: i64 = sqlx::query_scalar(
			"SELECT count(*) FROM information_schema.tables WHERE table_name = 'memory_space_grants'",
		)
		.fetch_one(&db.pool)
		.await
		.expect("Failed to query schema tables.");

		assert_eq!(count, 1);
	});
}

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn db_connects_and_bootstraps() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping db_connects_and_bootstraps; set ELF_PG_DSN to run this test.");

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn memory_space_grants_active_uniqueness_enforced() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!(
			"Skipping memory_space_grants_active_uniqueness_enforced; set ELF_PG_DSN to run."
		);

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	let project_grant = r#"
		INSERT INTO memory_space_grants (
			grant_id,
			tenant_id,
			project_id,
			scope,
			space_owner_agent_id,
			grantee_kind,
			grantee_agent_id,
			granted_by_agent_id
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
	"#;
	let first_project = sqlx::query(project_grant)
		.bind(Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid"))
		.bind("tenant_alpha")
		.bind("project_alpha")
		.bind("project_shared")
		.bind("owner_alpha")
		.bind("project")
		.bind(None::<String>)
		.bind("granter_alpha");
	let first_project_result = first_project.execute(&db.pool).await;

	assert!(
		first_project_result.is_ok(),
		"Expected first project grant to insert cleanly: {first_project_result:?}"
	);

	let duplicate_project = sqlx::query(project_grant)
		.bind(Uuid::parse_str("11111111-1111-1111-1111-111111111112").expect("uuid"))
		.bind("tenant_alpha")
		.bind("project_alpha")
		.bind("project_shared")
		.bind("owner_alpha")
		.bind("project")
		.bind(None::<String>)
		.bind("granter_alpha");

	assert!(duplicate_project.execute(&db.pool).await.is_err());

	let agent_grant = r#"
		INSERT INTO memory_space_grants (
			grant_id,
			tenant_id,
			project_id,
			scope,
			space_owner_agent_id,
			grantee_kind,
			grantee_agent_id,
			granted_by_agent_id
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
	"#;
	let first_agent = sqlx::query(agent_grant)
		.bind(Uuid::parse_str("22222222-2222-2222-2222-222222222221").expect("uuid"))
		.bind("tenant_alpha")
		.bind("project_alpha")
		.bind("project_shared")
		.bind("owner_alpha")
		.bind("agent")
		.bind("grantee_alpha")
		.bind("granter_alpha");

	assert!(first_agent.execute(&db.pool).await.is_ok());

	let duplicate_agent = sqlx::query(agent_grant)
		.bind(Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid"))
		.bind("tenant_alpha")
		.bind("project_alpha")
		.bind("project_shared")
		.bind("owner_alpha")
		.bind("agent")
		.bind("grantee_alpha")
		.bind("granter_alpha");

	assert!(duplicate_agent.execute(&db.pool).await.is_err());

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
