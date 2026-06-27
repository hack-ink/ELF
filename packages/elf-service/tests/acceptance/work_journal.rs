use std::sync::{Arc, atomic::AtomicUsize};

use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank};
use elf_domain::writegate::{WritePolicy, WriteRedaction, WriteSpan};
use elf_service::{
	ElfService, Error, Providers, WorkJournalEntryCreateRequest, WorkJournalEntryFamily,
	WorkJournalEntryGetRequest, WorkJournalSessionReadbackRequest,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;

fn work_journal_entry_request(entry_id: Uuid) -> WorkJournalEntryCreateRequest {
	WorkJournalEntryCreateRequest {
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		agent_id: "agent-a".to_string(),
		entry_id: Some(entry_id),
		scope: "agent_private".to_string(),
		session_id: "xy-1117-session".to_string(),
		family: WorkJournalEntryFamily::SessionLog,
		title: Some("XY-1117 session log".to_string()),
		body: "Work stopped after the dry run failed with api_key=abcdef123.".to_string(),
		source_refs: vec![serde_json::json!({
			"schema": "source_ref/v1",
			"resolver": "work_journal_test/v1",
			"ref": {
				"issue": "XY-1117",
				"session_id": "xy-1117-session"
			}
		})],
		write_policy: Some(WritePolicy {
			exclusions: vec![],
			redactions: vec![WriteRedaction::Replace {
				span: WriteSpan { start: 43, end: 60 },
				replacement: "[redacted credential]".to_string(),
			}],
		}),
		explicit_next_steps: vec!["Run the Work Journal validation tests.".to_string()],
		inferred_next_steps: vec![
			"Keep journal evidence separate from current memory answers.".to_string(),
		],
		rejected_options: vec![
			"Do not store this session log as an authoritative memory note.".to_string(),
		],
		promotion_boundary: serde_json::json!({ "authoritative_memory_allowed": true }),
	}
}

fn request_with_promotion_boundary(
	entry_id: Uuid,
	promotion_boundary: Value,
) -> WorkJournalEntryCreateRequest {
	let mut request = work_journal_entry_request(entry_id);

	request.body = "Work stopped after accepted promotion evidence was reviewed.".to_string();
	request.write_policy = None;
	request.promotion_boundary = promotion_boundary;

	request
}

fn memory_record_ref(note_id: Uuid) -> Value {
	serde_json::json!({
		"schema": "elf.memory_record_ref/v1",
		"kind": "note",
		"id": note_id,
		"status": "active"
	})
}

fn dreaming_review_ref(proposal_id: Uuid, review_state: &str) -> Value {
	serde_json::json!({
		"schema": "elf.dreaming_review_queue/v1",
		"proposal_id": proposal_id,
		"review_state": review_state
	})
}

async fn work_journal_service() -> Option<(ElfService, TestDatabase)> {
	let Some(dsn) = elf_testkit::env_dsn() else {
		eprintln!("Skipping work_journal acceptance; set ELF_PG_DSN to run this test.");

		return None;
	};
	let test_db = TestDatabase::new(&dsn).await.expect("Failed to create test database.");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		"http://127.0.0.1:1".to_string(),
		4_096,
		test_db.collection_name("elf_acceptance_notes"),
		test_db.collection_name("elf_acceptance_docs"),
	);
	let db = Db::connect(&cfg.storage.postgres).await.expect("Failed to connect test DB.");

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await.expect("Failed to ensure schema");

	let qdrant = QdrantStore::new(&cfg.storage.qdrant).expect("Failed to build qdrant store");
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({}),
		}),
	);
	let service = ElfService::with_providers(cfg, db, qdrant, providers);

	Some((service, test_db))
}

#[tokio::test]
async fn work_journal_persists_redacted_source_adjacent_session_readback() {
	let Some((service, test_db)) = work_journal_service().await else {
		return;
	};
	let entry_id = Uuid::parse_str("aaaaaaaa-1111-4111-8111-aaaaaaaa1111").expect("uuid");
	let created = service
		.work_journal_entry_create(work_journal_entry_request(entry_id))
		.await
		.expect("journal entry should persist");

	assert_eq!(created.entry.entry_id, entry_id);
	assert!(created.entry.body.contains("[redacted credential]"));
	assert!(!created.entry.body.contains("abcdef123"));
	assert_eq!(
		created.entry.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(false)
	);

	let fetched = service
		.work_journal_entry_get(WorkJournalEntryGetRequest {
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent-a".to_string(),
			read_profile: "private_only".to_string(),
			entry_id,
		})
		.await
		.expect("journal entry should be readable");

	assert_eq!(fetched.entry_id, entry_id);
	assert_eq!(fetched.source_refs.len(), 1);

	let readback = service
		.work_journal_session_readback(WorkJournalSessionReadbackRequest {
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent-a".to_string(),
			read_profile: "private_only".to_string(),
			session_id: "xy-1117-session".to_string(),
			families: vec![],
			limit: Some(10),
		})
		.await
		.expect("session readback should load journal evidence");

	assert_eq!(readback.items.len(), 1);

	let where_stopped = readback.where_stopped.expect("where_stopped should be present");

	assert_eq!(where_stopped.latest_entry_id, entry_id);
	assert_eq!(
		where_stopped.explicit_next_steps,
		vec!["Run the Work Journal validation tests.".to_string()]
	);
	assert_eq!(
		where_stopped.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(false)
	);

	let memory_count: i64 = sqlx::query_scalar("SELECT count(*) FROM memory_notes")
		.fetch_one(&service.db.pool)
		.await
		.expect("memory_notes count should query");
	let outbox_count: i64 = sqlx::query_scalar("SELECT count(*) FROM indexing_outbox")
		.fetch_one(&service.db.pool)
		.await
		.expect("indexing_outbox count should query");

	assert_eq!(memory_count, 0, "Work Journal must not create authoritative memory notes");
	assert_eq!(outbox_count, 0, "Work Journal must not enqueue memory indexing");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
async fn work_journal_promotion_boundary_requires_existing_accepted_refs() {
	let Some((service, test_db)) = work_journal_service().await else {
		return;
	};
	let forged_note_id = Uuid::parse_str("bbbbbbbb-1111-4111-8111-bbbbbbbb1111").expect("uuid");
	let forged_note_request = request_with_promotion_boundary(
		Uuid::parse_str("bbbbbbbb-2222-4222-8222-bbbbbbbb2222").expect("uuid"),
		serde_json::json!({
			"accepted_memory_authority_ref": memory_record_ref(forged_note_id),
		}),
	);
	let forged_note_error = service
		.work_journal_entry_create(forged_note_request)
		.await
		.expect_err("syntactically valid but nonexistent memory authority ref should be rejected");

	assert!(matches!(
		forged_note_error,
		Error::InvalidRequest { message } if message.contains("accepted_memory_authority_ref")
	));

	let accepted_note_id = Uuid::parse_str("cccccccc-1111-4111-8111-cccccccc1111").expect("uuid");

	insert_active_memory_note(&service, accepted_note_id).await;

	let accepted_note_request = request_with_promotion_boundary(
		Uuid::parse_str("cccccccc-2222-4222-8222-cccccccc2222").expect("uuid"),
		serde_json::json!({
			"accepted_memory_authority_ref": memory_record_ref(accepted_note_id),
		}),
	);
	let accepted_note = service
		.work_journal_entry_create(accepted_note_request)
		.await
		.expect("existing active memory authority ref should be accepted");

	assert_eq!(
		accepted_note.entry.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(true)
	);

	let forged_proposal_id = Uuid::parse_str("dddddddd-1111-4111-8111-dddddddd1111").expect("uuid");
	let forged_proposal_request = request_with_promotion_boundary(
		Uuid::parse_str("dddddddd-2222-4222-8222-dddddddd2222").expect("uuid"),
		serde_json::json!({
			"accepted_dreaming_review_ref": dreaming_review_ref(forged_proposal_id, "applied"),
		}),
	);
	let forged_proposal_error = service
		.work_journal_entry_create(forged_proposal_request)
		.await
		.expect_err("syntactically valid but nonexistent dreaming review ref should be rejected");

	assert!(matches!(
		forged_proposal_error,
		Error::InvalidRequest { message } if message.contains("accepted_dreaming_review_ref")
	));

	let accepted_proposal_id =
		Uuid::parse_str("eeeeeeee-1111-4111-8111-eeeeeeee1111").expect("uuid");

	insert_applied_dreaming_proposal(&service, accepted_proposal_id).await;

	let accepted_proposal_request = request_with_promotion_boundary(
		Uuid::parse_str("eeeeeeee-2222-4222-8222-eeeeeeee2222").expect("uuid"),
		serde_json::json!({
			"accepted_dreaming_review_ref": dreaming_review_ref(accepted_proposal_id, "applied"),
		}),
	);
	let accepted_proposal = service
		.work_journal_entry_create(accepted_proposal_request)
		.await
		.expect("existing applied dreaming review ref should be accepted");

	assert_eq!(
		accepted_proposal.entry.promotion_boundary["authoritative_memory_allowed"],
		serde_json::json!(true)
	);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn insert_active_memory_note(service: &ElfService, note_id: Uuid) {
	let now = OffsetDateTime::now_utc();

	sqlx::query(
		"\
INSERT INTO memory_notes (
	note_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref
)
VALUES ($1,'tenant','project','agent-a','agent_private','fact','accepted-memory-ref','Fact: The accepted memory note is active and readable.',0.8,0.9,'active',$2,$2,NULL,'test:embedding:4096',$3)",
	)
	.bind(note_id)
	.bind(now)
	.bind(serde_json::json!({ "schema": "work_journal_test/v1", "kind": "accepted_memory" }))
	.execute(&service.db.pool)
	.await
	.expect("accepted memory note should insert");
}

async fn insert_applied_dreaming_proposal(service: &ElfService, proposal_id: Uuid) {
	let run_id = Uuid::parse_str("eeeeeeee-3333-4333-8333-eeeeeeee3333").expect("uuid");
	let now = OffsetDateTime::now_utc();

	sqlx::query(
		"\
INSERT INTO consolidation_runs (
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	job_kind,
	status,
	input_refs,
	source_snapshot,
	lineage,
	error,
	created_at,
	updated_at,
	completed_at
)
VALUES ($1,'tenant','project','agent-a','elf.consolidation/v1','manual','completed',$2,$3,$4,'{}'::jsonb,$5,$5,$5)",
	)
	.bind(run_id)
	.bind(serde_json::json!([]))
	.bind(serde_json::json!({ "source_count": 0 }))
	.bind(serde_json::json!({ "source": "work_journal_test" }))
	.bind(now)
	.execute(&service.db.pool)
	.await
	.expect("consolidation run should insert");
	sqlx::query(
		"\
INSERT INTO consolidation_proposals (
	proposal_id,
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	proposal_kind,
	apply_intent,
	review_state,
	source_refs,
	source_snapshot,
	lineage,
	diff,
	confidence,
	unsupported_claim_flags,
	contradiction_markers,
	staleness_markers,
	target_ref,
	proposed_payload,
	reviewer_agent_id,
	review_comment,
	reviewed_at,
	created_at,
	updated_at
)
VALUES ($1,$2,'tenant','project','agent-a','elf.consolidation/v1','memory_summary','no_op','applied',$3,$4,$5,$6,0.9,'[]'::jsonb,'[]'::jsonb,'[]'::jsonb,'{}'::jsonb,$7,'agent-a','Apply reviewed Work Journal test proposal.',$8,$8,$8)",
	)
	.bind(proposal_id)
	.bind(run_id)
	.bind(serde_json::json!([]))
	.bind(serde_json::json!({ "source_count": 0 }))
	.bind(serde_json::json!({ "source": "work_journal_test" }))
	.bind(serde_json::json!({ "summary": "Applied proposal supports Work Journal authority." }))
	.bind(serde_json::json!({ "schema": "elf.dreaming_review_queue/v1" }))
	.bind(now)
	.execute(&service.db.pool)
	.await
	.expect("consolidation proposal should insert");
}
