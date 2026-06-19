//! Dreaming review queue readback over consolidation proposals.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ConsolidationProposalResponse, ConsolidationProposalReviewEventResponse, ElfService, Result,
};
use elf_domain::consolidation::ConsolidationReviewState;
use elf_storage::consolidation;

/// Schema identifier for Dreaming review queue responses.
pub const ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1: &str = "elf.dreaming_review_queue/v1";

const DEFAULT_QUEUE_LIMIT: u32 = 50;
const MAX_QUEUE_LIMIT: u32 = 200;
const HIGH_CONFIDENCE_AUTO_APPLY_FLOOR: f32 = 0.9;
const FORBIDDEN_SOURCE_MUTATION_KEYS: [&str; 8] = [
	"delete_source",
	"delete_sources",
	"overwrite_source",
	"source_delete",
	"source_mutation",
	"source_mutations",
	"source_note_updates",
	"update_source",
];

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

impl ElfService {
	/// Lists consolidation proposals as a Dreaming review queue.
	pub async fn dreaming_review_queue(
		&self,
		req: DreamingReviewQueueRequest,
	) -> Result<DreamingReviewQueueResponse> {
		let limit = bounded_queue_limit(req.limit);
		let review_state = req.review_state.map(ConsolidationReviewState::as_str);
		let proposals = consolidation::list_consolidation_proposals(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.run_id,
			review_state,
			limit,
		)
		.await?;
		let mut items = Vec::with_capacity(proposals.len());

		for proposal in proposals {
			let review_events = consolidation::list_consolidation_proposal_review_events(
				&self.db.pool,
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				proposal.proposal_id,
			)
			.await?
			.into_iter()
			.map(ConsolidationProposalReviewEventResponse::from)
			.collect();
			let mut response = ConsolidationProposalResponse::from(proposal);

			response.review_events = review_events;

			items.push(DreamingReviewQueueItem::from(response));
		}

		Ok(DreamingReviewQueueResponse {
			schema: ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1.to_string(),
			policy: DreamingReviewQueuePolicy::default(),
			summary: summarize_items(&items),
			items,
		})
	}
}

fn summarize_items(items: &[DreamingReviewQueueItem]) -> DreamingReviewQueueSummary {
	let mut summary = DreamingReviewQueueSummary {
		item_count: items.len(),
		..DreamingReviewQueueSummary::default()
	};
	let mut variants = BTreeSet::new();

	for item in items {
		match item.review_state.as_str() {
			"proposed" => summary.proposed_count += 1,
			"approved" => summary.approved_count += 1,
			"applied" => summary.applied_count += 1,
			"rejected" => summary.discarded_count += 1,
			"archived" => summary.deferred_count += 1,
			_ => {},
		}

		if item.policy.high_impact {
			summary.high_impact_count += 1;
		}
		if item.policy.source_mutation_requested {
			summary.source_mutation_requested_count += 1;
		}
		if item.policy.auto_apply_candidate {
			summary.auto_apply_candidate_count += 1;
		}
		if item.policy.auto_apply_allowed {
			summary.auto_apply_allowed_count += 1;
		}

		variants.insert(item.queue_variant.as_str());
	}

	summary.variant_count = variants.len();

	summary
}

fn queue_variant_for(proposal_kind: &str, apply_intent: &str, proposed_payload: &Value) -> String {
	for pointer in [
		"/queue_variant",
		"/dreaming_variant",
		"/proposal_variant",
		"/variant",
		"/artifact_kind",
		"/metadata/queue_variant",
		"/metadata/dreaming_variant",
		"/metadata/artifact_kind",
	] {
		if let Some(raw) = proposed_payload.pointer(pointer).and_then(Value::as_str)
			&& let Some(variant) = normalize_variant(raw)
		{
			return variant;
		}
	}

	if let Some(variant) = normalize_variant(proposal_kind) {
		return variant;
	}

	match apply_intent {
		"create_derived_knowledge_page" | "update_derived_knowledge_page" =>
			"page_rebuild".to_string(),
		"create_derived_graph_view" => "graph_fact".to_string(),
		"create_derived_note" | "update_derived_note" => "memory_promotion".to_string(),
		_ => "other".to_string(),
	}
}

fn normalize_variant(raw: &str) -> Option<String> {
	let token = raw.trim().to_ascii_lowercase().replace(['-', ' '], "_");

	if token.is_empty() {
		return None;
	}
	if token.contains("duplicate") || token.contains("dedupe") {
		return Some("duplicate_merge".to_string());
	}
	if token.contains("tag") || token.contains("taxonomy") {
		return Some("tag".to_string());
	}
	if token.contains("knowledge_page") || token.contains("page_rebuild") {
		return Some("page_rebuild".to_string());
	}
	if token.contains("graph_fact") || token.contains("graph_view") {
		return Some("graph_fact".to_string());
	}
	if token.contains("proactive_brief") || token.contains("daily_brief") {
		return Some("proactive_brief".to_string());
	}
	if token.contains("scheduled_memory") || token.contains("weekly_summary") {
		return Some("scheduled_memory".to_string());
	}
	if token.contains("memory_summary") || token.contains("summary") {
		return Some("memory_summary".to_string());
	}
	if token.contains("memory_promotion") || token.contains("derived_note") {
		return Some("memory_promotion".to_string());
	}
	if token.contains("correction") || token.contains("repair") {
		return Some("correction".to_string());
	}

	Some(token)
}

fn affected_refs(target_ref: &Value, proposed_payload: &Value) -> Vec<Value> {
	let mut refs = Vec::new();

	push_non_empty_object(&mut refs, target_ref);

	for pointer in [
		"/affected_refs",
		"/affected_pages",
		"/affected_memories",
		"/affected_facts",
		"/affected_notes",
	] {
		match proposed_payload.pointer(pointer) {
			Some(Value::Array(values)) => refs.extend(values.iter().cloned()),
			Some(value) if non_empty_json_object(value) => refs.push(value.clone()),
			_ => {},
		}
	}

	refs
}

fn push_non_empty_object(refs: &mut Vec<Value>, value: &Value) {
	if non_empty_json_object(value) {
		refs.push(value.clone());
	}
}

fn non_empty_json_object(value: &Value) -> bool {
	value.as_object().is_some_and(|object| !object.is_empty())
}

fn non_empty_json_array(value: &Value) -> bool {
	value.as_array().is_some_and(|array| !array.is_empty())
}

fn contains_forbidden_source_mutation_key(value: &Value) -> bool {
	match value {
		Value::Object(map) => map.iter().any(|(key, nested)| {
			FORBIDDEN_SOURCE_MUTATION_KEYS.contains(&key.as_str())
				|| contains_forbidden_source_mutation_key(nested)
		}),
		Value::Array(items) => items.iter().any(contains_forbidden_source_mutation_key),
		_ => false,
	}
}

fn low_risk_derived_organization(queue_variant: &str) -> bool {
	matches!(queue_variant, "tag" | "duplicate_merge")
}

fn high_impact_variant(queue_variant: &str) -> bool {
	matches!(queue_variant, "memory_promotion" | "graph_fact" | "correction")
}

fn available_review_actions(review_state: &str, manual_apply_allowed: bool) -> Vec<String> {
	let actions = match review_state {
		"proposed" => &["approve", "defer", "discard"][..],
		"approved" if manual_apply_allowed => &["apply", "defer", "discard"][..],
		"approved" => &["defer", "discard"][..],
		_ => &[][..],
	};

	actions.iter().map(|action| (*action).to_string()).collect()
}

fn policy_reason(
	source_mutation_requested: bool,
	high_impact: bool,
	has_unsupported_claims: bool,
	has_review_markers: bool,
	auto_apply_candidate: bool,
	auto_apply_allowed: bool,
	manual_apply_allowed: bool,
) -> String {
	if source_mutation_requested {
		return "source mutation is requested, so the proposal cannot be applied by the queue"
			.to_string();
	}
	if has_unsupported_claims || has_review_markers {
		return "lint or review markers require explicit reviewer inspection".to_string();
	}
	if auto_apply_allowed {
		return "approved low-risk derived organization proposal satisfies auto-apply policy"
			.to_string();
	}
	if manual_apply_allowed {
		return "approved review-gated proposal may be manually applied to a derived target"
			.to_string();
	}
	if high_impact {
		return "high-impact memory, graph, or correction proposal requires approval before apply"
			.to_string();
	}

	if auto_apply_candidate {
		return "low-risk derived organization proposal is a candidate after reviewer approval"
			.to_string();
	}

	"proposal remains reviewable derived output".to_string()
}

fn bounded_queue_limit(limit: Option<u32>) -> i64 {
	i64::from(limit.unwrap_or(DEFAULT_QUEUE_LIMIT).clamp(1, MAX_QUEUE_LIMIT))
}

#[cfg(test)]
mod tests {
	use serde_json;
	use time::OffsetDateTime;
	use uuid::Uuid;

	use crate::{ConsolidationProposalResponse, dreaming_review_queue};

	#[test]
	fn queue_variant_prefers_payload_and_normalizes_future_variants() {
		let payload = serde_json::json!({
			"metadata": { "queue_variant": "Duplicate Merge" }
		});

		assert_eq!(
			dreaming_review_queue::queue_variant_for(
				"derived_note",
				"create_derived_note",
				&payload
			),
			"duplicate_merge"
		);
		assert_eq!(
			dreaming_review_queue::queue_variant_for(
				"knowledge_page",
				"update_derived_knowledge_page",
				&serde_json::json!({})
			),
			"page_rebuild"
		);
		assert_eq!(
			dreaming_review_queue::queue_variant_for(
				"correction",
				"no_op",
				&serde_json::json!({ "affected_notes": [] })
			),
			"correction"
		);
	}

	#[test]
	fn policy_detects_source_mutation_and_review_actions() {
		assert!(dreaming_review_queue::contains_forbidden_source_mutation_key(
			&serde_json::json!({
				"after": { "source_note_updates": [{ "note_id": "n1" }] }
			})
		));
		assert_eq!(
			dreaming_review_queue::available_review_actions("proposed", false),
			vec!["approve", "defer", "discard"]
		);
		assert_eq!(
			dreaming_review_queue::available_review_actions("approved", true),
			vec!["apply", "defer", "discard"]
		);
		assert_eq!(
			dreaming_review_queue::available_review_actions("approved", false),
			vec!["defer", "discard"]
		);
		assert!(dreaming_review_queue::available_review_actions("applied", false).is_empty());
	}

	#[test]
	fn affected_refs_include_target_and_payload_refs() {
		let refs = dreaming_review_queue::affected_refs(
			&serde_json::json!({ "kind": "knowledge_page", "page_key": "architecture" }),
			&serde_json::json!({
				"affected_pages": [{ "page_key": "architecture" }],
				"affected_facts": [{ "fact_id": "f1" }]
			}),
		);

		assert_eq!(refs.len(), 3);
	}

	#[test]
	fn queue_item_policy_separates_review_apply_and_auto_apply() {
		let proposed_tag = dreaming_review_queue::DreamingReviewQueueItem::from(proposal(
			"tag",
			"no_op",
			"proposed",
			0.95,
			serde_json::json!({ "queue_variant": "tag" }),
			serde_json::json!({ "summary": "tag", "before": {}, "after": {} }),
		));

		assert!(proposed_tag.policy.auto_apply_candidate);
		assert!(!proposed_tag.policy.auto_apply_allowed);
		assert!(proposed_tag.policy.requires_review);
		assert_eq!(
			proposed_tag.review_audit.available_actions,
			vec!["approve", "defer", "discard"]
		);

		let approved_tag = dreaming_review_queue::DreamingReviewQueueItem::from(proposal(
			"tag",
			"no_op",
			"approved",
			0.95,
			serde_json::json!({ "queue_variant": "tag" }),
			serde_json::json!({ "summary": "tag", "before": {}, "after": {} }),
		));

		assert!(approved_tag.policy.auto_apply_allowed);
		assert!(!approved_tag.policy.requires_review);
		assert_eq!(approved_tag.review_audit.available_actions, vec!["apply", "defer", "discard"]);

		let approved_graph = dreaming_review_queue::DreamingReviewQueueItem::from(proposal(
			"graph_fact",
			"create_derived_graph_view",
			"approved",
			0.95,
			serde_json::json!({ "queue_variant": "graph_fact" }),
			serde_json::json!({ "summary": "graph", "before": {}, "after": {} }),
		));

		assert!(approved_graph.policy.high_impact);
		assert!(!approved_graph.policy.auto_apply_allowed);
		assert_eq!(
			approved_graph.review_audit.available_actions,
			vec!["apply", "defer", "discard"]
		);

		let source_mutation = dreaming_review_queue::DreamingReviewQueueItem::from(proposal(
			"tag",
			"no_op",
			"approved",
			0.95,
			serde_json::json!({ "queue_variant": "tag" }),
			serde_json::json!({
				"summary": "source mutation",
				"before": {},
				"after": { "source_mutation": true }
			}),
		));

		assert!(source_mutation.policy.source_mutation_requested);
		assert!(!source_mutation.policy.auto_apply_allowed);
		assert!(source_mutation.policy.requires_review);
		assert_eq!(source_mutation.review_audit.available_actions, vec!["defer", "discard"]);

		let memory_promotion = dreaming_review_queue::DreamingReviewQueueItem::from(proposal(
			"derived_note",
			"create_derived_note",
			"proposed",
			0.95,
			serde_json::json!({}),
			serde_json::json!({ "summary": "promote", "before": {}, "after": {} }),
		));

		assert_eq!(memory_promotion.queue_variant, "memory_promotion");
		assert!(memory_promotion.policy.high_impact);
		assert!(!memory_promotion.policy.auto_apply_candidate);
	}

	fn proposal(
		proposal_kind: &str,
		apply_intent: &str,
		review_state: &str,
		confidence: f32,
		proposed_payload: serde_json::Value,
		diff: serde_json::Value,
	) -> ConsolidationProposalResponse {
		let now = OffsetDateTime::UNIX_EPOCH;

		ConsolidationProposalResponse {
			proposal_id: Uuid::nil(),
			run_id: Uuid::nil(),
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent".to_string(),
			contract_schema: "elf.consolidation/v1".to_string(),
			proposal_kind: proposal_kind.to_string(),
			apply_intent: apply_intent.to_string(),
			review_state: review_state.to_string(),
			source_refs: serde_json::json!([]),
			source_snapshot: serde_json::json!({}),
			lineage: serde_json::json!({}),
			diff,
			confidence,
			unsupported_claim_flags: serde_json::json!([]),
			contradiction_markers: serde_json::json!([]),
			staleness_markers: serde_json::json!([]),
			target_ref: serde_json::json!({}),
			proposed_payload,
			reviewer_agent_id: None,
			review_comment: None,
			reviewed_at: None,
			created_at: now,
			updated_at: now,
			review_events: Vec::new(),
		}
	}
}
