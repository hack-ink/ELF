use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// Arguments for updating a consolidation run state.
pub struct ConsolidationRunStateUpdate<'a> {
	/// Tenant that owns the run.
	pub tenant_id: &'a str,
	/// Project that owns the run.
	pub project_id: &'a str,
	/// Run identifier.
	pub run_id: Uuid,
	/// New run status.
	pub status: &'a str,
	/// Structured error payload for terminal failure states.
	pub error: &'a Value,
	/// Update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for updating a consolidation proposal review state.
pub struct ConsolidationProposalReviewUpdate<'a> {
	/// Tenant that owns the proposal.
	pub tenant_id: &'a str,
	/// Project that owns the proposal.
	pub project_id: &'a str,
	/// Proposal identifier.
	pub proposal_id: Uuid,
	/// New review state.
	pub review_state: &'a str,
	/// Reviewing agent identifier.
	pub reviewer_agent_id: &'a str,
	/// Optional reviewer comment.
	pub review_comment: Option<&'a str>,
	/// Update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for updating a consolidation proposal target reference.
pub struct ConsolidationProposalTargetRefUpdate<'a> {
	/// Tenant that owns the proposal.
	pub tenant_id: &'a str,
	/// Project that owns the proposal.
	pub project_id: &'a str,
	/// Proposal identifier.
	pub proposal_id: Uuid,
	/// New target reference.
	pub target_ref: &'a Value,
	/// Update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting a consolidation proposal review event.
pub struct ConsolidationProposalReviewEventInsert<'a> {
	/// Review event identifier.
	pub review_id: Uuid,
	/// Reviewed proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: &'a str,
	/// Project that owns the proposal.
	pub project_id: &'a str,
	/// Reviewing agent identifier.
	pub reviewer_agent_id: &'a str,
	/// Review action requested by the reviewer.
	pub action: &'a str,
	/// Review state before the transition.
	pub from_review_state: &'a str,
	/// Review state after the transition.
	pub to_review_state: &'a str,
	/// Optional reviewer comment.
	pub review_comment: Option<&'a str>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Arguments for inserting a consolidation worker job.
pub struct ConsolidationRunJobInsert<'a> {
	/// Worker job identifier.
	pub job_id: Uuid,
	/// Consolidation run to materialize.
	pub run_id: Uuid,
	/// Tenant that owns the run.
	pub tenant_id: &'a str,
	/// Project that owns the run.
	pub project_id: &'a str,
	/// Agent that registered the run.
	pub agent_id: &'a str,
	/// Job kind, such as fixture or manual.
	pub job_kind: &'a str,
	/// Queued proposal payload.
	pub payload: &'a Value,
	/// Creation timestamp.
	pub now: OffsetDateTime,
}
