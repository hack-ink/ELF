use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_storage::models::{ConsolidationProposal, ConsolidationProposalReviewEvent};

/// Response returned by consolidation proposal listing.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationProposalsListResponse {
	/// Returned proposals.
	pub proposals: Vec<ConsolidationProposalResponse>,
}

/// Public consolidation proposal review audit DTO.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationProposalReviewEventResponse {
	/// Review event identifier.
	pub review_id: Uuid,
	/// Reviewed proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent that performed the review action.
	pub reviewer_agent_id: String,
	/// Review action requested by the reviewer.
	pub action: String,
	/// Review state before the transition.
	pub from_review_state: String,
	/// Review state after the transition.
	pub to_review_state: String,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<ConsolidationProposalReviewEvent> for ConsolidationProposalReviewEventResponse {
	fn from(event: ConsolidationProposalReviewEvent) -> Self {
		Self {
			review_id: event.review_id,
			proposal_id: event.proposal_id,
			run_id: event.run_id,
			tenant_id: event.tenant_id,
			project_id: event.project_id,
			reviewer_agent_id: event.reviewer_agent_id,
			action: event.action,
			from_review_state: event.from_review_state,
			to_review_state: event.to_review_state,
			review_comment: event.review_comment,
			created_at: event.created_at,
		}
	}
}

/// Public consolidation proposal DTO.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationProposalResponse {
	/// Consolidation proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent that registered the proposal.
	pub agent_id: String,
	/// Versioned consolidation contract schema.
	pub contract_schema: String,
	/// Proposal kind, such as derived_note or knowledge_page.
	pub proposal_kind: String,
	/// Derived-output apply intent.
	pub apply_intent: String,
	/// Current review state.
	pub review_state: String,
	/// Serialized source references.
	pub source_refs: Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: Value,
	/// Serialized proposal lineage.
	pub lineage: Value,
	/// Serialized reviewable diff.
	pub diff: Value,
	/// Proposal confidence score.
	pub confidence: f32,
	/// Serialized unsupported-claim flags.
	pub unsupported_claim_flags: Value,
	/// Serialized contradiction markers.
	pub contradiction_markers: Value,
	/// Serialized staleness markers.
	pub staleness_markers: Value,
	/// Serialized derived target reference.
	pub target_ref: Value,
	/// Serialized proposed derived output payload.
	pub proposed_payload: Value,
	/// Agent that last reviewed the proposal.
	pub reviewer_agent_id: Option<String>,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
	/// Timestamp of the last review transition.
	pub reviewed_at: Option<OffsetDateTime>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Append-only review events for detail readback.
	pub review_events: Vec<ConsolidationProposalReviewEventResponse>,
}
impl From<ConsolidationProposal> for ConsolidationProposalResponse {
	fn from(proposal: ConsolidationProposal) -> Self {
		Self {
			proposal_id: proposal.proposal_id,
			run_id: proposal.run_id,
			tenant_id: proposal.tenant_id,
			project_id: proposal.project_id,
			agent_id: proposal.agent_id,
			contract_schema: proposal.contract_schema,
			proposal_kind: proposal.proposal_kind,
			apply_intent: proposal.apply_intent,
			review_state: proposal.review_state,
			source_refs: proposal.source_refs,
			source_snapshot: proposal.source_snapshot,
			lineage: proposal.lineage,
			diff: proposal.diff,
			confidence: proposal.confidence,
			unsupported_claim_flags: proposal.unsupported_claim_flags,
			contradiction_markers: proposal.contradiction_markers,
			staleness_markers: proposal.staleness_markers,
			target_ref: proposal.target_ref,
			proposed_payload: proposal.proposed_payload,
			reviewer_agent_id: proposal.reviewer_agent_id,
			review_comment: proposal.review_comment,
			reviewed_at: proposal.reviewed_at,
			created_at: proposal.created_at,
			updated_at: proposal.updated_at,
			review_events: Vec::new(),
		}
	}
}
