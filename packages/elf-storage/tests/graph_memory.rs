use sqlx::PgConnection;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use elf_config::Postgres;
use elf_storage::{
	db::Db,
	models::{GraphFact, MemoryNote},
	queries,
};
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
	let entity_id = elf_storage::graph::upsert_entity(
		&mut tx,
		tenant_id,
		project_id,
		"  Alice   Doe ",
		Some("person"),
	)
	.await
	.expect("Failed to upsert canonical entity.");
	let canonical_norm = elf_storage::graph::normalize_entity_name("Alice doe");

	assert_eq!(canonical_norm, "alice doe");

	let entity_again = elf_storage::graph::upsert_entity(
		&mut tx,
		tenant_id,
		project_id,
		"Alice\tDoe",
		Some("person"),
	)
	.await
	.expect("Failed to upsert canonical alias.");

	assert_eq!(entity_id, entity_again);

	tx.commit().await.expect("Failed to commit transaction.");

	assert!(test_db.cleanup().await.is_ok(), "Failed to cleanup test database.");
}

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
	let subject =
		elf_storage::graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity A", None)
			.await
			.expect("Failed to upsert subject.");
	let predicate = elf_storage::graph::resolve_or_register_predicate(
		&mut tx,
		"tenant-a",
		"project-a",
		"related_to",
	)
	.await
	.expect("Failed to resolve predicate.");
	let err = elf_storage::graph::insert_fact_with_evidence(
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
	let note_id = insert_memory_note(&mut tx, "tenant-a", "project-a").await;
	let subject =
		elf_storage::graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
			.await
			.expect("Failed to upsert subject.");
	let object =
		elf_storage::graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Object", None)
			.await
			.expect("Failed to upsert object.");
	let predicate = elf_storage::graph::resolve_or_register_predicate(
		&mut tx,
		"tenant-a",
		"project-a",
		"related_to",
	)
	.await
	.expect("Failed to resolve predicate.");
	let now = OffsetDateTime::now_utc();

	elf_storage::graph::insert_fact_with_evidence(
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

	let err = elf_storage::graph::insert_fact_with_evidence(
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
	let note_id = insert_memory_note(&mut tx, "tenant-a", "project-a").await;
	let subject =
		elf_storage::graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
			.await
			.expect("Failed to upsert subject.");
	let predicate = elf_storage::graph::resolve_or_register_predicate(
		&mut tx,
		"tenant-a",
		"project-a",
		"expires",
	)
	.await
	.expect("Failed to resolve predicate.");
	let now = OffsetDateTime::now_utc();
	let err = elf_storage::graph::insert_fact_with_evidence(
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
	let note_id = insert_memory_note(&mut tx, "tenant-a", "project-a").await;
	let subject =
		elf_storage::graph::upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
			.await
			.expect("Failed to upsert subject.");
	let active_predicate = elf_storage::graph::resolve_or_register_predicate(
		&mut tx,
		"tenant-a",
		"project-a",
		"active_fact",
	)
	.await
	.expect("Failed to resolve predicate.");
	let expired_predicate = elf_storage::graph::resolve_or_register_predicate(
		&mut tx,
		"tenant-a",
		"project-a",
		"expired_fact",
	)
	.await
	.expect("Failed to resolve predicate.");
	let future_predicate = elf_storage::graph::resolve_or_register_predicate(
		&mut tx,
		"tenant-a",
		"project-a",
		"future_fact",
	)
	.await
	.expect("Failed to resolve predicate.");
	let now = OffsetDateTime::now_utc();
	let active = elf_storage::graph::insert_fact_with_evidence(
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

	elf_storage::graph::insert_fact_with_evidence(
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
	elf_storage::graph::insert_fact_with_evidence(
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

	let facts: Vec<GraphFact> = elf_storage::graph::fetch_active_facts_for_subject(
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
	let predicate = elf_storage::graph::resolve_or_register_predicate(
		&mut tx,
		"tenant-a",
		"project-a",
		"mentors",
	)
	.await
	.expect("Failed to resolve predicate.");
	let updated_active = elf_storage::graph::update_predicate_guarded(
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
	let updated_deprecated = elf_storage::graph::update_predicate_guarded(
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

	let err = elf_storage::graph::update_predicate_guarded(
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

	let predicate_now = elf_storage::graph::get_predicate_by_id(&mut tx, predicate.predicate_id)
		.await
		.expect("Failed to load predicate.")
		.expect("Expected predicate row.");

	assert_eq!(predicate_now.status, "deprecated");

	tx.rollback().await.expect("Failed to rollback transaction.");
	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn insert_memory_note(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
) -> Uuid {
	let note_id = Uuid::new_v4();
	let note = MemoryNote {
		note_id,
		tenant_id: tenant_id.to_string(),
		project_id: project_id.to_string(),
		agent_id: "agent-a".to_string(),
		scope: "scope-a".to_string(),
		r#type: "fact".to_string(),
		key: None,
		text: "graph note evidence".to_string(),
		importance: 1.0,
		confidence: 1.0,
		status: "active".to_string(),
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		expires_at: None,
		embedding_version: "test:vec:1".to_string(),
		source_ref: serde_json::json!({}),
		hit_count: 0,
		last_hit_at: None,
	};

	queries::insert_note(executor, &note).await.expect("Failed to insert evidence note.");

	note_id
}
