use std::sync::{Arc, atomic::AtomicUsize};

use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank};
use elf_chunking::ChunkingConfig;
use elf_domain::consolidation::{
	ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
	ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
	ConsolidationReviewAction, ConsolidationSourceKind, ConsolidationSourceSnapshot,
	ConsolidationUnsupportedClaimFlag,
};
use elf_service::{
	AddNoteInput, AddNoteRequest, ConsolidationProposalGetRequest, ConsolidationProposalInput,
	ConsolidationProposalReviewRequest, ConsolidationProposalsListRequest,
	ConsolidationProposalsListResponse, ConsolidationRunCreateRequest,
	ConsolidationRunCreateResponse, ConsolidationRunGetRequest, ElfService, ListRequest,
	MemoryCorrectionAction, MemoryCorrectionRequest, MemoryCorrectionResponse,
	MemoryHistoryGetRequest, Providers,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker::{self, WorkerState};

const TENANT_ID: &str = "tenant_consolidation";
const PROJECT_ID: &str = "project_consolidation";
const AGENT_ID: &str = "agent_consolidation";
const REVIEWER_ID: &str = "reviewer_consolidation";

struct ConsolidationFixture {
	service: ElfService,
	_test_db: TestDatabase,
}

fn source_ref(note_id: Uuid) -> ConsolidationInputRef {
	ConsolidationInputRef {
		kind: ConsolidationSourceKind::Note,
		id: note_id,
		snapshot: ConsolidationSourceSnapshot {
			status: Some("active".to_string()),
			updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			content_hash: Some("blake3:acceptance-source".to_string()),
			embedding_version: Some("test:test:4096".to_string()),
			trace_version: None,
			source_ref: serde_json::json!({ "schema": "acceptance/v1" }),
			metadata: serde_json::json!({ "fixture": "consolidation" }),
		},
	}
}

fn lineage(source: &ConsolidationInputRef) -> ConsolidationLineage {
	ConsolidationLineage {
		source_refs: vec![source.clone()],
		parent_run_id: None,
		parent_proposal_ids: Vec::new(),
	}
}

fn proposal_input(source: &ConsolidationInputRef, kind: &str) -> ConsolidationProposalInput {
	proposal_input_with_payload(
		source,
		kind,
		serde_json::json!({
			"type": "fact",
			"text": "Fact: Consolidation proposals are derived and reviewable."
		}),
	)
}

fn proposal_input_with_payload(
	source: &ConsolidationInputRef,
	kind: &str,
	proposed_payload: serde_json::Value,
) -> ConsolidationProposalInput {
	ConsolidationProposalInput {
		proposal_kind: kind.to_string(),
		apply_intent: ConsolidationApplyIntent::CreateDerivedNote,
		source_refs: vec![source.clone()],
		source_snapshot: serde_json::json!({ "source_count": 1 }),
		lineage: lineage(source),
		confidence: 0.82,
		unsupported_claim_flags: vec![ConsolidationUnsupportedClaimFlag {
			claim_id: Some("unsupported-claim".to_string()),
			message: "The source does not prove that source notes may be rewritten.".to_string(),
			source: Some(source.clone()),
		}],
		markers: ConsolidationMarkers {
			contradictions: vec![ConsolidationMarker {
				severity: ConsolidationMarkerSeverity::High,
				message: "Stale rewrite evidence conflicts with the proposal-only rule."
					.to_string(),
				source: Some(source.clone()),
			}],
			staleness: Vec::new(),
		},
		diff: ConsolidationProposalDiff {
			summary: "Create a reviewed derived note without changing source evidence.".to_string(),
			before: serde_json::json!({}),
			after: serde_json::json!({
				"target": "derived_note",
				"text": "Fact: Consolidation proposals are derived and reviewable."
			}),
		},
		target_ref: serde_json::json!({}),
		proposed_payload,
	}
}

fn proposal_id_by_kind(response: &ConsolidationProposalsListResponse, proposal_kind: &str) -> Uuid {
	response
		.proposals
		.iter()
		.find(|proposal| proposal.proposal_kind == proposal_kind)
		.map(|proposal| proposal.proposal_id)
		.expect("proposal kind should be present")
}

async fn setup_service(test_name: &str) -> Option<ConsolidationFixture> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let extractor = SpyExtractor {
		calls: Arc::new(AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let service =
		acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	Some(ConsolidationFixture { service, _test_db: test_db })
}

async fn insert_source_note(service: &ElfService, key: &str, text: &str) -> Uuid {
	let response = service
		.add_note(AddNoteRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some(key.to_string()),
				text: text.to_string(),
				structured: None,
				importance: 0.7,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({ "schema": "acceptance/v1", "key": key }),
				write_policy: None,
			}],
		})
		.await
		.expect("add_note should persist source note");

	response.results[0].note_id.expect("source note id should be present")
}

async fn create_run_with_proposals(
	service: &ElfService,
	source: &ConsolidationInputRef,
	proposals: Vec<ConsolidationProposalInput>,
) -> ConsolidationRunCreateResponse {
	service
		.consolidation_run_create(ConsolidationRunCreateRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			job_kind: "manual".to_string(),
			input_refs: vec![source.clone()],
			source_snapshot: serde_json::json!({ "source_count": 1 }),
			lineage: lineage(source),
			proposals,
		})
		.await
		.expect("consolidation run should be created")
}

async fn process_consolidation_worker(service: &ElfService) {
	let tokenizer = elf_chunking::load_tokenizer(&service.cfg.chunking.tokenizer_repo)
		.expect("worker tokenizer should load");
	let mut embedding = acceptance::dummy_embedding_provider();

	embedding.dimensions = service.cfg.storage.qdrant.vector_dim;

	let worker_state = WorkerState {
		db: Db::connect(&service.cfg.storage.postgres).await.expect("Failed to connect worker DB."),
		qdrant: QdrantStore::new(&service.cfg.storage.qdrant)
			.expect("Failed to build Qdrant store."),
		docs_qdrant: QdrantStore::new_with_collection(
			&service.cfg.storage.qdrant,
			&service.cfg.storage.qdrant.docs_collection,
		)
		.expect("Failed to build docs Qdrant store."),
		embedding,
		chunking: ChunkingConfig {
			max_tokens: service.cfg.chunking.max_tokens,
			overlap_tokens: service.cfg.chunking.overlap_tokens,
		},
		tokenizer,
	};

	worker::process_once(&worker_state).await.expect("consolidation worker should process once");
}

async fn materialized_proposals(
	service: &ElfService,
	run_id: Uuid,
) -> ConsolidationProposalsListResponse {
	service
		.consolidation_proposals_list(ConsolidationProposalsListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			run_id: Some(run_id),
			review_state: None,
			limit: None,
		})
		.await
		.expect("consolidation proposals should be listed")
}

async fn promote_reviewed_memory(service: &ElfService) -> Uuid {
	let note_id = insert_source_note(
		service,
		"memory_authority_source",
		"Fact: Reviewed memories require source-linked approval.",
	)
	.await;
	let source = source_ref(note_id);
	let created =
		create_run_with_proposals(service, &source, vec![proposal_input(&source, "derived_note")])
			.await;

	process_consolidation_worker(service).await;

	let materialized = materialized_proposals(service, created.run.run_id).await;
	let proposal_id = materialized.proposals[0].proposal_id;
	let reviewed = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: AGENT_ID.to_string(),
			proposal_id,
			review_action: ConsolidationReviewAction::Apply,
			review_comment: Some("Approve memory authority candidate.".to_string()),
		})
		.await
		.expect("review action should promote memory");

	reviewed
		.target_ref
		.get("id")
		.and_then(serde_json::Value::as_str)
		.and_then(|value| Uuid::parse_str(value).ok())
		.expect("applied proposal should point at promoted note")
}

async fn active_list_contains(service: &ElfService, note_id: Uuid) -> bool {
	service
		.list(ListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: Some(AGENT_ID.to_string()),
			scope: Some("agent_private".to_string()),
			status: None,
			r#type: None,
		})
		.await
		.expect("active notes should list")
		.items
		.iter()
		.any(|item| item.note_id == note_id)
}

async fn apply_memory_correction(
	service: &ElfService,
	note_id: Uuid,
	action: MemoryCorrectionAction,
	reason: &str,
	source: &str,
	restore_version_id: Option<Uuid>,
) -> MemoryCorrectionResponse {
	service
		.memory_correction_apply(MemoryCorrectionRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			actor_agent_id: AGENT_ID.to_string(),
			note_id,
			action,
			reason: reason.to_string(),
			source_ref: serde_json::json!({
				"schema": "acceptance/review",
				"source": source
			}),
			restore_version_id,
		})
		.await
		.expect("memory correction should persist")
}

async fn memory_history_event_types(service: &ElfService, note_id: Uuid) -> Vec<String> {
	service
		.memory_history_get(MemoryHistoryGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			note_id,
		})
		.await
		.expect("promoted memory history should be readable")
		.events
		.into_iter()
		.map(|event| event.event_type)
		.collect()
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn apply_action_is_audited_without_source_rewrite() {
	let Some(fixture) = setup_service("apply_action_is_audited_without_source_rewrite").await
	else {
		return;
	};
	let service = &fixture.service;
	let source_text =
		"Fact: Current consolidation output is derived and never rewrites source notes.";
	let note_id = insert_source_note(service, "consolidation_source_rule", source_text).await;
	let source = source_ref(note_id);
	let created =
		create_run_with_proposals(service, &source, vec![proposal_input(&source, "derived_note")])
			.await;

	assert_eq!(created.run.status, "pending");
	assert!(created.proposals.is_empty());

	process_consolidation_worker(service).await;

	let completed = service
		.consolidation_run_get(ConsolidationRunGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			run_id: created.run.run_id,
		})
		.await
		.expect("consolidation run should remain readable");
	let materialized = materialized_proposals(service, created.run.run_id).await;
	let proposal = &materialized.proposals[0];
	let job_status: String =
		sqlx::query_scalar("SELECT status FROM consolidation_run_jobs WHERE job_id = $1")
			.bind(created.job_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("consolidation job should be queryable");

	assert_eq!(completed.status, "completed");
	assert_eq!(job_status, "DONE");
	assert_eq!(materialized.proposals.len(), 1);
	assert_eq!(proposal.review_state, "proposed");
	assert_eq!(proposal.unsupported_claim_flags.as_array().map(Vec::len), Some(1));
	assert_eq!(proposal.contradiction_markers.as_array().map(Vec::len), Some(1));

	let reviewed = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: REVIEWER_ID.to_string(),
			proposal_id: proposal.proposal_id,
			review_action: ConsolidationReviewAction::Apply,
			review_comment: Some("Apply reviewed derived proposal.".to_string()),
		})
		.await
		.expect("review action should apply");

	assert_eq!(reviewed.review_state, "applied");
	assert_eq!(reviewed.review_events.len(), 2);
	assert_eq!(reviewed.review_events[0].action, "approve");
	assert_eq!(reviewed.review_events[0].from_review_state, "proposed");
	assert_eq!(reviewed.review_events[0].to_review_state, "approved");
	assert_eq!(reviewed.review_events[1].action, "apply");
	assert_eq!(reviewed.review_events[1].from_review_state, "approved");
	assert_eq!(reviewed.review_events[1].to_review_state, "applied");

	let promoted_note_id = reviewed
		.target_ref
		.get("id")
		.and_then(serde_json::Value::as_str)
		.and_then(|value| Uuid::parse_str(value).ok())
		.expect("applied proposal should point at promoted note");
	let promoted_source_ref: serde_json::Value =
		sqlx::query_scalar("SELECT source_ref FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory source ref should be queryable");
	let promoted_status: String =
		sqlx::query_scalar("SELECT status FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory status should be queryable");
	let promoted_agent_id: String =
		sqlx::query_scalar("SELECT agent_id FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory owner should be queryable");

	assert_eq!(promoted_status, "active");
	assert_eq!(promoted_agent_id, AGENT_ID);
	assert_eq!(promoted_source_ref["schema"], "elf.memory_promotion/v1");
	assert_eq!(
		promoted_source_ref["proposal_id"].as_str().map(str::to_string),
		Some(proposal.proposal_id.to_string())
	);
	assert_eq!(promoted_source_ref["review"]["reviewer_agent_id"], REVIEWER_ID);

	let stored_text: String =
		sqlx::query_scalar("SELECT text FROM memory_notes WHERE note_id = $1")
			.bind(note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("source note should still exist");
	let version_count: i64 =
		sqlx::query_scalar("SELECT count(*) FROM memory_note_versions WHERE note_id = $1")
			.bind(note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("source note versions should be queryable");

	assert_eq!(stored_text, source_text);
	assert_eq!(version_count, 1);
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn apply_project_shared_memory_creates_owner_grant() {
	let Some(fixture) = setup_service("apply_project_shared_memory_creates_owner_grant").await
	else {
		return;
	};
	let service = &fixture.service;
	let note_id = insert_source_note(
		service,
		"consolidation_project_shared_source",
		"Fact: Shared memory promotions must preserve project grant semantics.",
	)
	.await;
	let source = source_ref(note_id);
	let proposal = proposal_input_with_payload(
		&source,
		"derived_note",
		serde_json::json!({
			"type": "fact",
			"scope": "project_shared",
			"text": "Fact: Project-shared promoted memories keep project grants."
		}),
	);
	let created = create_run_with_proposals(service, &source, vec![proposal]).await;

	process_consolidation_worker(service).await;

	let materialized = materialized_proposals(service, created.run.run_id).await;
	let reviewed = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: REVIEWER_ID.to_string(),
			proposal_id: materialized.proposals[0].proposal_id,
			review_action: ConsolidationReviewAction::Apply,
			review_comment: Some("Apply reviewed project-shared memory.".to_string()),
		})
		.await
		.expect("project-shared review action should promote memory");
	let promoted_note_id = reviewed
		.target_ref
		.get("id")
		.and_then(serde_json::Value::as_str)
		.and_then(|value| Uuid::parse_str(value).ok())
		.expect("applied proposal should point at promoted note");
	let promoted: (String, String, String) =
		sqlx::query_as("SELECT project_id, agent_id, scope FROM memory_notes WHERE note_id = $1")
			.bind(promoted_note_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("promoted memory should be queryable");
	let grant_count: i64 = sqlx::query_scalar(
		"\
SELECT count(*)
FROM memory_space_grants
WHERE tenant_id = $1
	AND project_id = $2
	AND scope = 'project_shared'
	AND space_owner_agent_id = $3
	AND grantee_kind = 'project'
	AND revoked_at IS NULL",
	)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(AGENT_ID)
	.fetch_one(&service.db.pool)
	.await
	.expect("project grant should be queryable");

	assert_eq!(
		promoted,
		(PROJECT_ID.to_string(), AGENT_ID.to_string(), "project_shared".to_string())
	);
	assert_eq!(grant_count, 1);
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn promoted_memory_corrections_suppress_and_restore_recall() {
	let Some(fixture) =
		setup_service("promoted_memory_corrections_suppress_and_restore_recall").await
	else {
		return;
	};
	let service = &fixture.service;
	let promoted_note_id = promote_reviewed_memory(service).await;
	let superseded = apply_memory_correction(
		service,
		promoted_note_id,
		MemoryCorrectionAction::Supersede,
		"Newer reviewed source supersedes the derived memory.",
		"supersede",
		None,
	)
	.await;

	assert_eq!(superseded.status, "deprecated");
	assert!(!active_list_contains(service, promoted_note_id).await);

	let restored = apply_memory_correction(
		service,
		promoted_note_id,
		MemoryCorrectionAction::Restore,
		"Rollback to prior approved memory after reviewer audit.",
		"restore",
		superseded.version_id,
	)
	.await;

	assert_eq!(restored.status, "active");
	assert!(active_list_contains(service, promoted_note_id).await);

	let deleted = apply_memory_correction(
		service,
		promoted_note_id,
		MemoryCorrectionAction::Delete,
		"Reviewer removed the restored memory from normal recall.",
		"delete",
		None,
	)
	.await;

	assert_eq!(deleted.status, "deleted");
	assert!(!active_list_contains(service, promoted_note_id).await);

	let event_types = memory_history_event_types(service, promoted_note_id).await;

	for expected in ["add", "derived", "applied", "superseded", "restored", "delete"] {
		assert!(event_types.iter().any(|event_type| event_type == expected));
	}
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn discard_and_defer_actions_remain_auditable() {
	let Some(fixture) = setup_service("discard_and_defer_actions_remain_auditable").await else {
		return;
	};
	let service = &fixture.service;
	let note_id = insert_source_note(
		service,
		"consolidation_review_actions",
		"Fact: Discarded and deferred proposals remain auditable.",
	)
	.await;
	let source = source_ref(note_id);
	let created = create_run_with_proposals(
		service,
		&source,
		vec![
			proposal_input(&source, "contradiction_report"),
			proposal_input(&source, "preference_candidate"),
		],
	)
	.await;

	process_consolidation_worker(service).await;

	let materialized = materialized_proposals(service, created.run.run_id).await;
	let discarded_id = proposal_id_by_kind(&materialized, "contradiction_report");
	let deferred_id = proposal_id_by_kind(&materialized, "preference_candidate");
	let discarded = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: AGENT_ID.to_string(),
			proposal_id: discarded_id,
			review_action: ConsolidationReviewAction::Discard,
			review_comment: Some("Discard stale synthesis.".to_string()),
		})
		.await
		.expect("discard should be allowed");
	let deferred = service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			reviewer_agent_id: AGENT_ID.to_string(),
			proposal_id: deferred_id,
			review_action: ConsolidationReviewAction::Defer,
			review_comment: Some("Defer until more evidence is available.".to_string()),
		})
		.await
		.expect("defer should be allowed");
	let deferred_readback = service
		.consolidation_proposal_get(ConsolidationProposalGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			proposal_id: deferred_id,
		})
		.await
		.expect("deferred proposal should remain readable");

	assert_eq!(discarded.review_state, "rejected");
	assert_eq!(discarded.review_events.len(), 1);
	assert_eq!(discarded.review_events[0].action, "discard");
	assert_eq!(deferred.review_state, "archived");
	assert_eq!(deferred.review_events.len(), 1);
	assert_eq!(deferred.review_events[0].action, "defer");
	assert_eq!(deferred_readback.review_events.len(), 1);
	assert_eq!(deferred_readback.review_events[0].to_review_state, "archived");
}
