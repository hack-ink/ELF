use elf_config::Postgres;
use elf_storage::{db::Db, graph};
use elf_testkit::TestDatabase;

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn graph_predicate_guarded_update_conflicts_after_deprecate() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!(
			"Skipping graph_predicate_guarded_update_conflicts_after_deprecate; set ELF_PG_DSN to run."
		);

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	let mut tx = db.pool.begin().await.expect("Failed to open transaction.");
	let predicate =
		graph::resolve_or_register_predicate(&mut tx, "tenant-a", "project-a", "mentors")
			.await
			.expect("Failed to resolve predicate.");
	let updated_active = graph::update_predicate_guarded(
		&mut tx,
		predicate.predicate_id,
		predicate.status.as_str(),
		predicate.cardinality.as_str(),
		Some("active"),
		None,
	)
	.await
	.expect("Failed to activate predicate.");
	let stale_expected_status = updated_active.status.clone();
	let stale_expected_cardinality = updated_active.cardinality.clone();
	let updated_deprecated = graph::update_predicate_guarded(
		&mut tx,
		predicate.predicate_id,
		updated_active.status.as_str(),
		updated_active.cardinality.as_str(),
		Some("deprecated"),
		None,
	)
	.await
	.expect("Failed to deprecate predicate.");

	assert_eq!(updated_deprecated.status, "deprecated");

	let err = graph::update_predicate_guarded(
		&mut tx,
		predicate.predicate_id,
		stale_expected_status.as_str(),
		stale_expected_cardinality.as_str(),
		None,
		Some("single"),
	)
	.await
	.expect_err("Expected guarded update to conflict after deprecate.");

	assert!(matches!(err, elf_storage::Error::Conflict(_)));

	let predicate_now = graph::get_predicate_by_id(&mut tx, predicate.predicate_id)
		.await
		.expect("Failed to load predicate.")
		.expect("Expected predicate row.");

	assert_eq!(predicate_now.status, "deprecated");

	tx.rollback().await.expect("Failed to rollback transaction.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
