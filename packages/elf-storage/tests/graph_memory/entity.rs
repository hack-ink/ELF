use elf_config::Postgres;
use elf_storage::{db::Db, graph};
use elf_testkit::TestDatabase;

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn graph_entity_upsert_is_idempotent_by_normalized_canonical() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!(
			"Skipping graph_entity_upsert_is_idempotent_by_normalized_canonical; set ELF_PG_DSN to run."
		);

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	let mut tx = db.pool.begin().await.expect("Failed to open transaction.");
	let tenant_id = "tenant-a";
	let project_id = "project-a";
	let entity_id =
		graph::upsert_entity(&mut tx, tenant_id, project_id, "  Alice   Doe ", Some("person"))
			.await
			.expect("Failed to upsert canonical entity.");
	let canonical_norm = graph::normalize_entity_name("Alice doe");

	assert_eq!(canonical_norm, "alice doe");

	let entity_again =
		graph::upsert_entity(&mut tx, tenant_id, project_id, "Alice\tDoe", Some("person"))
			.await
			.expect("Failed to upsert canonical alias.");

	assert_eq!(entity_id, entity_again);

	tx.commit().await.expect("Failed to commit transaction.");

	assert!(test_db.cleanup().await.is_ok(), "Failed to cleanup test database.");
}
