use crate::ConsolidationProposalResponse;

use super::{
	policy::{
		HIGH_CONFIDENCE_AUTO_APPLY_FLOOR, affected_refs, available_review_actions,
		contains_forbidden_source_mutation_key, high_impact_variant, low_risk_derived_organization,
		non_empty_json_array, policy_reason, queue_variant_for,
	},
	types::{DreamingReviewQueueAudit, DreamingReviewQueueItem, DreamingReviewQueueItemPolicy},
};

impl From<ConsolidationProposalResponse> for DreamingReviewQueueItem {
	fn from(proposal: ConsolidationProposalResponse) -> Self {
		let queue_variant = queue_variant_for(
			proposal.proposal_kind.as_str(),
			proposal.apply_intent.as_str(),
			&proposal.proposed_payload,
		);
		let source_mutation_requested = contains_forbidden_source_mutation_key(&proposal.diff)
			|| contains_forbidden_source_mutation_key(&proposal.proposed_payload)
			|| contains_forbidden_source_mutation_key(&proposal.target_ref);
		let high_impact = high_impact_variant(queue_variant.as_str());
		let has_unsupported_claims = non_empty_json_array(&proposal.unsupported_claim_flags);
		let has_review_markers = non_empty_json_array(&proposal.contradiction_markers)
			|| non_empty_json_array(&proposal.staleness_markers);
		let auto_apply_candidate = low_risk_derived_organization(queue_variant.as_str())
			&& proposal.confidence >= HIGH_CONFIDENCE_AUTO_APPLY_FLOOR
			&& !has_unsupported_claims
			&& !has_review_markers
			&& !source_mutation_requested;
		let manual_apply_allowed =
			proposal.review_state.as_str() == "approved" && !source_mutation_requested;
		let auto_apply_allowed = auto_apply_candidate && manual_apply_allowed;
		let requires_review = source_mutation_requested
			|| !matches!(proposal.review_state.as_str(), "approved" | "applied");
		let policy = DreamingReviewQueueItemPolicy {
			source_mutation_requested,
			high_impact,
			requires_review,
			auto_apply_candidate,
			auto_apply_allowed,
			reason: policy_reason(
				source_mutation_requested,
				high_impact,
				has_unsupported_claims,
				has_review_markers,
				auto_apply_candidate,
				auto_apply_allowed,
				manual_apply_allowed,
			),
		};
		let review_audit = DreamingReviewQueueAudit {
			review_state: proposal.review_state.clone(),
			available_actions: available_review_actions(
				proposal.review_state.as_str(),
				manual_apply_allowed,
			),
			reviewer_agent_id: proposal.reviewer_agent_id.clone(),
			review_comment: proposal.review_comment.clone(),
			reviewed_at: proposal.reviewed_at,
			review_events: proposal.review_events.clone(),
		};

		Self {
			proposal_id: proposal.proposal_id,
			run_id: proposal.run_id,
			proposal_kind: proposal.proposal_kind,
			queue_variant,
			apply_intent: proposal.apply_intent,
			review_state: proposal.review_state,
			source_refs: proposal.source_refs,
			source_snapshot: proposal.source_snapshot,
			affected_refs: affected_refs(&proposal.target_ref, &proposal.proposed_payload),
			target_ref: proposal.target_ref,
			diff: proposal.diff,
			confidence: proposal.confidence,
			unsupported_claim_flags: proposal.unsupported_claim_flags,
			contradiction_markers: proposal.contradiction_markers,
			staleness_markers: proposal.staleness_markers,
			proposed_payload: proposal.proposed_payload,
			policy,
			review_audit,
			created_at: proposal.created_at,
			updated_at: proposal.updated_at,
		}
	}
}
