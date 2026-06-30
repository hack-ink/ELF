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
	AddNoteInput, AddNoteRequest, ConsolidationProposalInput, ConsolidationProposalReviewRequest,
	ConsolidationProposalsListRequest, ConsolidationProposalsListResponse,
	ConsolidationRunCreateRequest, ConsolidationRunCreateResponse, ElfService, ListRequest,
	MemoryCorrectionAction, MemoryCorrectionRequest, MemoryCorrectionResponse,
	MemoryHistoryGetRequest, Providers,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker::{self, WorkerState};

pub(super) const TENANT_ID: &str = "tenant_consolidation";
pub(super) const PROJECT_ID: &str = "project_consolidation";
pub(super) const AGENT_ID: &str = "agent_consolidation";
pub(super) const REVIEWER_ID: &str = "reviewer_consolidation";

pub(super) struct ConsolidationFixture {
	pub(super) service: ElfService,
	_test_db: TestDatabase,
}

pub(super) fn source_ref(note_id: Uuid) -> ConsolidationInputRef {
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

pub(super) fn lineage(source: &ConsolidationInputRef) -> ConsolidationLineage {
	ConsolidationLineage {
		source_refs: vec![source.clone()],
		parent_run_id: None,
		parent_proposal_ids: Vec::new(),
	}
}

pub(super) fn proposal_input(
	source: &ConsolidationInputRef,
	kind: &str,
) -> ConsolidationProposalInput {
	proposal_input_with_payload(
		source,
		kind,
		serde_json::json!({
			"type": "fact",
			"text": "Fact: Consolidation proposals are derived and reviewable."
		}),
	)
}

pub(super) fn proposal_input_with_payload(
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

pub(super) fn proposal_id_by_kind(
	response: &ConsolidationProposalsListResponse,
	proposal_kind: &str,
) -> Uuid {
	response
		.proposals
		.iter()
		.find(|proposal| proposal.proposal_kind == proposal_kind)
		.map(|proposal| proposal.proposal_id)
		.expect("proposal kind should be present")
}

pub(super) async fn setup_service(test_name: &str) -> Option<ConsolidationFixture> {
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

pub(super) async fn insert_source_note(service: &ElfService, key: &str, text: &str) -> Uuid {
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

pub(super) async fn create_run_with_proposals(
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

pub(super) async fn process_consolidation_worker(service: &ElfService) {
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

pub(super) async fn materialized_proposals(
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

pub(super) async fn promote_reviewed_memory(service: &ElfService) -> Uuid {
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

pub(super) async fn active_list_contains(service: &ElfService, note_id: Uuid) -> bool {
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

pub(super) async fn apply_memory_correction(
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

pub(super) async fn memory_history_event_types(service: &ElfService, note_id: Uuid) -> Vec<String> {
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
