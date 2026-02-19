use serde_json::json;
use sqlx::PgConnection;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use elf_config::Postgres;
use elf_storage::{
	Error as StorageError,
	db::Db,
	graph::{
		fetch_active_facts_for_subject, insert_fact_with_evidence, normalize_entity_name,
		upsert_entity,
	},
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
	let entity_id = upsert_entity(&mut tx, tenant_id, project_id, "  Alice   Doe ", Some("person"))
		.await
		.expect("Failed to upsert canonical entity.");
	let canonical_norm = normalize_entity_name("Alice doe");
	assert_eq!(canonical_norm, "alice doe");

	let entity_again = upsert_entity(&mut tx, tenant_id, project_id, "Alice\tDoe", Some("person"))
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
	let subject = upsert_entity(&mut tx, "tenant-a", "project-a", "Entity A", None)
		.await
		.expect("Failed to upsert subject.");

	let err = insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"related_to",
		None,
		Some("value"),
		OffsetDateTime::now_utc(),
		None,
		&[],
	)
	.await
	.expect_err("Expected empty evidence to be rejected.");

	assert!(matches!(err, StorageError::InvalidArgument(_)));

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

	let subject = upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
		.await
		.expect("Failed to upsert subject.");
	let object = upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Object", None)
		.await
		.expect("Failed to upsert object.");

	let now = OffsetDateTime::now_utc();

	insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"related_to",
		Some(object),
		None,
		now,
		None,
		&[note_id],
	)
	.await
	.expect("Failed to insert graph fact.");

	let err = insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"related_to",
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

	let subject = upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
		.await
		.expect("Failed to upsert subject.");

	let now = OffsetDateTime::now_utc();
	let err = insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"expires",
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

	let subject = upsert_entity(&mut tx, "tenant-a", "project-a", "Entity Subject", None)
		.await
		.expect("Failed to upsert subject.");

	let now = OffsetDateTime::now_utc();

	let active = insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"active_fact",
		None,
		Some("alpha"),
		now - Duration::hours(1),
		None,
		&[note_id],
	)
	.await
	.expect("Failed to insert active graph fact.");

	insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"expired_fact",
		None,
		Some("beta"),
		now - Duration::hours(2),
		Some(now - Duration::minutes(1)),
		&[note_id],
	)
	.await
	.expect("Failed to insert expired graph fact.");

	insert_fact_with_evidence(
		&mut tx,
		"tenant-a",
		"project-a",
		"agent-a",
		"scope-a",
		subject,
		"future_fact",
		None,
		Some("gamma"),
		now + Duration::hours(1),
		None,
		&[note_id],
	)
	.await
	.expect("Failed to insert future graph fact.");

	let facts: Vec<GraphFact> =
		fetch_active_facts_for_subject(&mut tx, "tenant-a", "project-a", "scope-a", subject, now)
			.await
			.expect("Failed to fetch active graph facts.");

	assert_eq!(facts.len(), 1);
	assert_eq!(facts[0].fact_id, active);
	assert_eq!(facts[0].predicate, "active_fact");

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
		source_ref: json!({}),
		hit_count: 0,
		last_hit_at: None,
	};

	queries::insert_note(executor, &note).await.expect("Failed to insert evidence note.");

	note_id
}
