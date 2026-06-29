use std::collections::BTreeSet;

use serde_json::Value;

use super::types::{DreamingReviewQueueItem, DreamingReviewQueueSummary};

/// Schema identifier for Dreaming review queue responses.
pub const ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1: &str = "elf.dreaming_review_queue/v1";

const DEFAULT_QUEUE_LIMIT: u32 = 50;
const MAX_QUEUE_LIMIT: u32 = 200;
pub(super) const HIGH_CONFIDENCE_AUTO_APPLY_FLOOR: f32 = 0.9;
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

pub(super) fn summarize_items(items: &[DreamingReviewQueueItem]) -> DreamingReviewQueueSummary {
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

pub(super) fn queue_variant_for(
	proposal_kind: &str,
	apply_intent: &str,
	proposed_payload: &Value,
) -> String {
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

pub(super) fn affected_refs(target_ref: &Value, proposed_payload: &Value) -> Vec<Value> {
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

pub(super) fn non_empty_json_array(value: &Value) -> bool {
	value.as_array().is_some_and(|array| !array.is_empty())
}

pub(super) fn contains_forbidden_source_mutation_key(value: &Value) -> bool {
	match value {
		Value::Object(map) => map.iter().any(|(key, nested)| {
			FORBIDDEN_SOURCE_MUTATION_KEYS.contains(&key.as_str())
				|| contains_forbidden_source_mutation_key(nested)
		}),
		Value::Array(items) => items.iter().any(contains_forbidden_source_mutation_key),
		_ => false,
	}
}

pub(super) fn low_risk_derived_organization(queue_variant: &str) -> bool {
	matches!(queue_variant, "tag" | "duplicate_merge")
}

pub(super) fn high_impact_variant(queue_variant: &str) -> bool {
	matches!(queue_variant, "memory_promotion" | "graph_fact" | "correction")
}

pub(super) fn available_review_actions(
	review_state: &str,
	manual_apply_allowed: bool,
) -> Vec<String> {
	let actions = match review_state {
		"proposed" => &["approve", "defer", "discard"][..],
		"approved" if manual_apply_allowed => &["apply", "defer", "discard"][..],
		"approved" => &["defer", "discard"][..],
		_ => &[][..],
	};

	actions.iter().map(|action| (*action).to_string()).collect()
}

pub(super) fn policy_reason(
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

pub(super) fn bounded_queue_limit(limit: Option<u32>) -> i64 {
	i64::from(limit.unwrap_or(DEFAULT_QUEUE_LIMIT).clamp(1, MAX_QUEUE_LIMIT))
}
