use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::ConsolidationProposalReviewEventResponse;
use elf_domain::consolidation::ConsolidationReviewState;

/// Request payload for Dreaming review queue readback.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DreamingReviewQueueRequest {
	/// Tenant that owns the review queue.
	pub tenant_id: String,
	/// Project that owns the review queue.
	pub project_id: String,
	/// Optional run filter.
	pub run_id: Option<Uuid>,
	/// Optional review-state filter.
	pub review_state: Option<ConsolidationReviewState>,
	/// Maximum number of queue items to return.
	pub limit: Option<u32>,
}

/// Dreaming review queue response.
#[derive(Clone, Debug, Serialize)]
pub struct DreamingReviewQueueResponse {
	/// Response schema identifier.
	pub schema: String,
	/// Queue policy applied to every returned item.
	pub policy: DreamingReviewQueuePolicy,
	/// Aggregate queue summary.
	pub summary: DreamingReviewQueueSummary,
	/// Returned queue items.
	pub items: Vec<DreamingReviewQueueItem>,
}

/// Global review queue policy.
#[derive(Clone, Debug, Serialize)]
pub struct DreamingReviewQueuePolicy {
	/// Authoritative source mutation is never allowed by this queue surface.
	pub source_mutation_allowed: bool,
	/// Whether high-impact proposals require explicit review.
	pub high_impact_requires_review: bool,
	/// Low-risk derived organization variants that may become auto-apply candidates.
	pub low_risk_derived_organization_variants: Vec<String>,
	/// Review actions supported by the underlying consolidation proposal lifecycle.
	pub review_actions: Vec<String>,
	/// Human-readable policy summary.
	pub summary: String,
}
impl Default for DreamingReviewQueuePolicy {
	fn default() -> Self {
		Self {
			source_mutation_allowed: false,
			high_impact_requires_review: true,
			low_risk_derived_organization_variants: vec![
				"tag".to_string(),
				"duplicate_merge".to_string(),
			],
			review_actions: vec![
				"approve".to_string(),
				"apply".to_string(),
				"defer".to_string(),
				"discard".to_string(),
			],
			summary: "Dreaming review queue proposals are source-backed derived outputs; authoritative source mutation is disallowed, and high-impact memory or graph changes remain review-gated.".to_string(),
		}
	}
}

/// Aggregate queue summary.
#[derive(Clone, Debug, Default, Serialize)]
pub struct DreamingReviewQueueSummary {
	/// Returned item count.
	pub item_count: usize,
	/// Items still waiting for review.
	pub proposed_count: usize,
	/// Items approved but not marked applied.
	pub approved_count: usize,
	/// Items marked applied to derived targets.
	pub applied_count: usize,
	/// Items discarded by review.
	pub discarded_count: usize,
	/// Items deferred for later audit.
	pub deferred_count: usize,
	/// Items classified as high impact.
	pub high_impact_count: usize,
	/// Items that request source mutation and therefore cannot be auto-applied.
	pub source_mutation_requested_count: usize,
	/// Items eligible for low-risk derived organization auto-apply after approval.
	pub auto_apply_candidate_count: usize,
	/// Items that currently satisfy the queue's auto-apply policy.
	pub auto_apply_allowed_count: usize,
	/// Number of distinct queue variants represented by the response.
	pub variant_count: usize,
}

/// One Dreaming review queue item.
#[derive(Clone, Debug, Serialize)]
pub struct DreamingReviewQueueItem {
	/// Consolidation proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Consolidation proposal kind.
	pub proposal_kind: String,
	/// Dreaming queue variant inferred from proposal metadata.
	pub queue_variant: String,
	/// Derived-output apply intent.
	pub apply_intent: String,
	/// Current review state.
	pub review_state: String,
	/// Source references supporting the proposal.
	pub source_refs: Value,
	/// Aggregate immutable source snapshot.
	pub source_snapshot: Value,
	/// Target affected by the proposal, when supplied.
	pub target_ref: Value,
	/// Affected pages, memories, facts, or derived artifacts extracted for reviewer scan.
	pub affected_refs: Vec<Value>,
	/// Reviewable diff.
	pub diff: Value,
	/// Proposal confidence.
	pub confidence: f32,
	/// Unsupported-claim lint flags.
	pub unsupported_claim_flags: Value,
	/// Contradiction markers for review.
	pub contradiction_markers: Value,
	/// Staleness markers for review.
	pub staleness_markers: Value,
	/// Proposed derived payload.
	pub proposed_payload: Value,
	/// Per-item policy decision.
	pub policy: DreamingReviewQueueItemPolicy,
	/// Review audit readback.
	pub review_audit: DreamingReviewQueueAudit,
	#[serde(with = "crate::time_serde")]
	/// Item creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Item update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Per-item policy readback.
#[derive(Clone, Debug, Serialize)]
pub struct DreamingReviewQueueItemPolicy {
	/// Whether this proposal requests mutation of authoritative sources.
	pub source_mutation_requested: bool,
	/// Whether this item is considered high impact.
	pub high_impact: bool,
	/// Whether reviewer approval is required before downstream application.
	pub requires_review: bool,
	/// Whether this item is a low-risk derived organization auto-apply candidate.
	pub auto_apply_candidate: bool,
	/// Whether this item currently satisfies auto-apply policy.
	pub auto_apply_allowed: bool,
	/// Reason for the policy decision.
	pub reason: String,
}

/// Review audit readback for one queue item.
#[derive(Clone, Debug, Serialize)]
pub struct DreamingReviewQueueAudit {
	/// Current review state.
	pub review_state: String,
	/// Actions currently accepted by the consolidation proposal lifecycle.
	pub available_actions: Vec<String>,
	/// Agent that last reviewed the item.
	pub reviewer_agent_id: Option<String>,
	/// Last reviewer comment.
	pub review_comment: Option<String>,
	#[serde(with = "crate::time_serde::option")]
	/// Last review timestamp.
	pub reviewed_at: Option<OffsetDateTime>,
	/// Append-only review events.
	pub review_events: Vec<ConsolidationProposalReviewEventResponse>,
}
