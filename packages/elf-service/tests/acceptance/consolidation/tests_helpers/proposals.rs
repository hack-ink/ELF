use serde_json::Value;
use uuid::Uuid;

use crate::acceptance::consolidation::tests_helpers::{AGENT_ID, PROJECT_ID, TENANT_ID, refs};
use elf_domain::consolidation::{
	ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationMarker,
	ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
	ConsolidationUnsupportedClaimFlag,
};
use elf_service::{
	ConsolidationProposalInput, ConsolidationProposalsListRequest,
	ConsolidationProposalsListResponse, ConsolidationRunCreateRequest,
	ConsolidationRunCreateResponse, ElfService,
};

pub(in crate::acceptance::consolidation) fn proposal_input(
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

pub(in crate::acceptance::consolidation) fn proposal_input_with_payload(
	source: &ConsolidationInputRef,
	kind: &str,
	proposed_payload: Value,
) -> ConsolidationProposalInput {
	ConsolidationProposalInput {
		proposal_kind: kind.to_string(),
		apply_intent: ConsolidationApplyIntent::CreateDerivedNote,
		source_refs: vec![source.clone()],
		source_snapshot: serde_json::json!({ "source_count": 1 }),
		lineage: refs::lineage(source),
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

pub(in crate::acceptance::consolidation) fn proposal_id_by_kind(
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

pub(in crate::acceptance::consolidation) async fn create_run_with_proposals(
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
			lineage: refs::lineage(source),
			proposals,
		})
		.await
		.expect("consolidation run should be created")
}

pub(in crate::acceptance::consolidation) async fn materialized_proposals(
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
