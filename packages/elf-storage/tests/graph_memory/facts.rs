use time::{Duration, OffsetDateTime};

use crate::graph_memory::helpers;
use elf_config::Postgres;
use elf_storage::{db::Db, graph, models::GraphFact};
use elf_testkit::TestDatabase;

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn graph_fact_with_empty_evidence_is_rejected() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping graph_fact_with_empty_evidence_is_rejected; set ELF_PG_DSN to run.");

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	let mut tx = db.pool.begin().await.expect("Failed to open transaction.");
	let subject = graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity A", None)
		.await
		.expect("Failed to upsert subject.");
	let predicate =
		graph::resolve_or_register_predicate(&mut tx, "tenant-a", "project-a", "related_to")
			.await
			.expect("Failed to resolve predicate.");
	let err = graph::insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"related_to",
		predicate.predicate_id,
		None,
		Some("value"),
		OffsetDateTime::now_utc(),
		None,
		&[],
	)
	.await
	.expect_err("Expected empty evidence to be rejected.");

	assert!(matches!(err, elf_storage::Error::InvalidArgument(_)));

	tx.rollback().await.expect("Failed to rollback transaction.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn graph_fact_duplicates_with_active_window_fail_unique_constraint() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!(
			"Skipping graph_fact_duplicates_with_active_window_fail_unique_constraint; set ELF_PG_DSN to run."
		);

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	let mut tx = db.pool.begin().await.expect("Failed to open transaction.");
	let note_id = helpers::insert_memory_note(&mut tx, "tenant-a", "project-a").await;
	let subject = graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
		.await
		.expect("Failed to upsert subject.");
	let object = graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Object", None)
		.await
		.expect("Failed to upsert object.");
	let predicate =
		graph::resolve_or_register_predicate(&mut tx, "tenant-a", "project-a", "related_to")
			.await
			.expect("Failed to resolve predicate.");
	let now = OffsetDateTime::now_utc();

	graph::insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"related_to",
		predicate.predicate_id,
		Some(object),
		None,
		now,
		None,
		&[note_id],
	)
	.await
	.expect("Failed to insert graph fact.");

	let err = graph::insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"related_to",
		predicate.predicate_id,
		Some(object),
		None,
		now,
		None,
		&[note_id],
	)
	.await;

	assert!(err.is_err());

	tx.rollback().await.expect("Failed to rollback transaction.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn graph_fact_rejects_invalid_valid_window() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping graph_fact_rejects_invalid_valid_window; set ELF_PG_DSN to run.");

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	let mut tx = db.pool.begin().await.expect("Failed to open transaction.");
	let note_id = helpers::insert_memory_note(&mut tx, "tenant-a", "project-a").await;
	let subject = graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
		.await
		.expect("Failed to upsert subject.");
	let predicate =
		graph::resolve_or_register_predicate(&mut tx, "tenant-a", "project-a", "expires")
			.await
			.expect("Failed to resolve predicate.");
	let now = OffsetDateTime::now_utc();
	let err = graph::insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"expires",
		predicate.predicate_id,
		None,
		Some("value"),
		now,
		Some(now),
		&[note_id],
	)
	.await;

	assert!(err.is_err());

	tx.rollback().await.expect("Failed to rollback transaction.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres. Set ELF_PG_DSN to run."]
async fn graph_fetch_active_facts_returns_active_window_only() {
	let Some(base_dsn) = elf_testkit::env_dsn() else {
		eprintln!(
			"Skipping graph_fetch_active_facts_returns_active_window_only; set ELF_PG_DSN to run."
		);

		return;
	};
	let test_db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
	let cfg = Postgres { dsn: test_db.dsn().to_string(), pool_max_conns: 1 };
	let db = Db::connect(&cfg).await.expect("Failed to connect to Postgres.");

	db.ensure_schema(4_096).await.expect("Failed to ensure schema.");

	let mut tx = db.pool.begin().await.expect("Failed to open transaction.");
	let note_id = helpers::insert_memory_note(&mut tx, "tenant-a", "project-a").await;
	let subject = graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
		.await
		.expect("Failed to upsert subject.");
	let active_predicate =
		graph::resolve_or_register_predicate(&mut tx, "tenant-a", "project-a", "active_fact")
			.await
			.expect("Failed to resolve predicate.");
	let expired_predicate =
		graph::resolve_or_register_predicate(&mut tx, "tenant-a", "project-a", "expired_fact")
			.await
			.expect("Failed to resolve predicate.");
	let future_predicate =
		graph::resolve_or_register_predicate(&mut tx, "tenant-a", "project-a", "future_fact")
			.await
			.expect("Failed to resolve predicate.");
	let now = OffsetDateTime::now_utc();
	let active = graph::insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"active_fact",
		active_predicate.predicate_id,
		None,
		Some("alpha"),
		now - Duration::hours(1),
		None,
		&[note_id],
	)
	.await
	.expect("Failed to insert active graph fact.");

	graph::insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"expired_fact",
		expired_predicate.predicate_id,
		None,
		Some("beta"),
		now - Duration::hours(2),
		Some(now - Duration::minutes(1)),
		&[note_id],
	)
	.await
	.expect("Failed to insert expired graph fact.");
	graph::insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"future_fact",
		future_predicate.predicate_id,
		None,
		Some("gamma"),
		now + Duration::hours(1),
		None,
		&[note_id],
	)
	.await
	.expect("Failed to insert future graph fact.");

	let facts: Vec<GraphFact> = graph::fetch_active_facts_for_subject(
		&mut tx,
		"tenant-a",
		"project-a",
		"scope-a",
		subject,
		now,
	)
	.await
	.expect("Failed to fetch active graph facts.");

	assert_eq!(facts.len(), 1);
	assert_eq!(facts[0].fact_id, active);
	assert_eq!(facts[0].predicate, "active_fact");

	tx.rollback().await.expect("Failed to rollback transaction.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
