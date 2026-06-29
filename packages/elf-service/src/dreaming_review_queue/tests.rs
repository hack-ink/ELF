use serde_json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::ConsolidationProposalResponse;

use super::{DreamingReviewQueueItem, policy};

#[test]
fn queue_variant_prefers_payload_and_normalizes_future_variants() {
	let payload = serde_json::json!({
		"metadata": { "queue_variant": "Duplicate Merge" }
	});

	assert_eq!(
		policy::queue_variant_for("derived_note", "create_derived_note", &payload),
		"duplicate_merge"
	);
	assert_eq!(
		policy::queue_variant_for(
			"knowledge_page",
			"update_derived_knowledge_page",
			&serde_json::json!({})
		),
		"page_rebuild"
	);
	assert_eq!(
		policy::queue_variant_for(
			"correction",
			"no_op",
			&serde_json::json!({ "affected_notes": [] })
		),
		"correction"
	);
}

#[test]
fn policy_detects_source_mutation_and_review_actions() {
	assert!(policy::contains_forbidden_source_mutation_key(&serde_json::json!({
		"after": { "source_note_updates": [{ "note_id": "n1" }] }
	})));
	assert_eq!(
		policy::available_review_actions("proposed", false),
		vec!["approve", "defer", "discard"]
	);
	assert_eq!(
		policy::available_review_actions("approved", true),
		vec!["apply", "defer", "discard"]
	);
	assert_eq!(policy::available_review_actions("approved", false), vec!["defer", "discard"]);
	assert!(policy::available_review_actions("applied", false).is_empty());
}

#[test]
fn affected_refs_include_target_and_payload_refs() {
	let refs = policy::affected_refs(
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
	let proposed_tag = DreamingReviewQueueItem::from(proposal(
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
	assert_eq!(proposed_tag.review_audit.available_actions, vec!["approve", "defer", "discard"]);

	let approved_tag = DreamingReviewQueueItem::from(proposal(
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

	let approved_graph = DreamingReviewQueueItem::from(proposal(
		"graph_fact",
		"create_derived_graph_view",
		"approved",
		0.95,
		serde_json::json!({ "queue_variant": "graph_fact" }),
		serde_json::json!({ "summary": "graph", "before": {}, "after": {} }),
	));

	assert!(approved_graph.policy.high_impact);
	assert!(!approved_graph.policy.auto_apply_allowed);
	assert_eq!(approved_graph.review_audit.available_actions, vec!["apply", "defer", "discard"]);

	let source_mutation = DreamingReviewQueueItem::from(proposal(
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

	let memory_promotion = DreamingReviewQueueItem::from(proposal(
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
