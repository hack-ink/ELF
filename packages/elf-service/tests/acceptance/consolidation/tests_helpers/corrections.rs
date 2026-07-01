use serde_json::Value;
use uuid::Uuid;

use crate::acceptance::consolidation::tests_helpers::{
	AGENT_ID, PROJECT_ID, TENANT_ID, notes, proposals, refs, worker_processing,
};
use elf_domain::consolidation::ConsolidationReviewAction;
use elf_service::{
	ConsolidationProposalReviewRequest, ElfService, ListRequest, MemoryCorrectionAction,
	MemoryCorrectionRequest, MemoryCorrectionResponse, MemoryHistoryGetRequest,
};

pub(in crate::acceptance::consolidation) async fn promote_reviewed_memory(
	service: &ElfService,
) -> Uuid {
	let note_id = notes::insert_source_note(
		service,
		"memory_authority_source",
		"Fact: Reviewed memories require source-linked approval.",
	)
	.await;
	let source = refs::source_ref(note_id);
	let created = proposals::create_run_with_proposals(
		service,
		&source,
		vec![proposals::proposal_input(&source, "derived_note")],
	)
	.await;

	worker_processing::process_consolidation_worker(service).await;

	let materialized = proposals::materialized_proposals(service, created.run.run_id).await;
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
		.and_then(Value::as_str)
		.and_then(|value| Uuid::parse_str(value).ok())
		.expect("applied proposal should point at promoted note")
}

pub(in crate::acceptance::consolidation) async fn active_list_contains(
	service: &ElfService,
	note_id: Uuid,
) -> bool {
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

pub(in crate::acceptance::consolidation) async fn apply_memory_correction(
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

pub(in crate::acceptance::consolidation) async fn memory_history_event_types(
	service: &ElfService,
	note_id: Uuid,
) -> Vec<String> {
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
