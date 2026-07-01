use std::sync::{Arc, atomic::AtomicUsize};

use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank};
use elf_domain::writegate::{WritePolicy, WriteRedaction, WriteSpan};
use elf_service::{ElfService, Providers, WorkJournalEntryCreateRequest, WorkJournalEntryFamily};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;

pub(super) fn work_journal_entry_request(entry_id: Uuid) -> WorkJournalEntryCreateRequest {
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

pub(super) fn request_with_promotion_boundary(
	entry_id: Uuid,
	promotion_boundary: Value,
) -> WorkJournalEntryCreateRequest {
	let mut request = work_journal_entry_request(entry_id);

	request.body = "Work stopped after accepted promotion evidence was reviewed.".to_string();
	request.write_policy = None;
	request.promotion_boundary = promotion_boundary;

	request
}

pub(super) fn memory_record_ref(note_id: Uuid) -> Value {
	serde_json::json!({
		"schema": "elf.memory_record_ref/v1",
		"kind": "note",
		"id": note_id,
		"status": "active"
	})
}

pub(super) fn dreaming_review_ref(proposal_id: Uuid, review_state: &str) -> Value {
	serde_json::json!({
		"schema": "elf.dreaming_review_queue/v1",
		"proposal_id": proposal_id,
		"review_state": review_state
	})
}

pub(super) async fn work_journal_service() -> Option<(ElfService, TestDatabase)> {
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

pub(super) async fn insert_active_memory_note(service: &ElfService, note_id: Uuid) {
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

pub(super) async fn insert_applied_dreaming_proposal(service: &ElfService, proposal_id: Uuid) {
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
