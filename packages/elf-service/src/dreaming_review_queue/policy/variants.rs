use serde_json::Value;

pub(in crate::dreaming_review_queue) fn queue_variant_for(
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

pub(in crate::dreaming_review_queue) fn low_risk_derived_organization(queue_variant: &str) -> bool {
	matches!(queue_variant, "tag" | "duplicate_merge")
}

pub(in crate::dreaming_review_queue) fn high_impact_variant(queue_variant: &str) -> bool {
	matches!(queue_variant, "memory_promotion" | "graph_fact" | "correction")
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
