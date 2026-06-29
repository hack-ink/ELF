use super::*;

pub(in crate::knowledge) fn low_source_coverage_finding(page: &KnowledgePage) -> LintDraft {
	LintDraft {
		section_id: None,
		finding_type: "low_source_coverage".to_string(),
		severity: "warning".to_string(),
		source_kind: None,
		source_id: None,
		message: "Knowledge page source coverage is incomplete.".to_string(),
		details: serde_json::json!({
			"source_coverage": page.source_coverage.clone(),
			"repair_guidance": repair_guidance_for_finding_type("low_source_coverage"),
		}),
	}
}

pub(in crate::knowledge) fn with_repair_guidance(
	details: Value,
	section_key: &str,
	guidance: &str,
) -> Value {
	let mut object = details.as_object().cloned().unwrap_or_default();

	object.insert("section_key".to_string(), Value::String(section_key.to_string()));
	object.insert("repair_guidance".to_string(), Value::String(guidance.to_string()));

	Value::Object(object)
}

pub(in crate::knowledge) fn missing_source_finding(
	source_ref: &KnowledgePageSourceRef,
) -> LintDraft {
	LintDraft {
		section_id: source_ref.section_id,
		finding_type: "stale_source_ref".to_string(),
		severity: "error".to_string(),
		source_kind: KnowledgeSourceKind::parse(source_ref.source_kind.as_str()),
		source_id: Some(source_ref.source_id),
		message: "Knowledge page source reference no longer resolves.".to_string(),
		details: serde_json::json!({
			"source_kind": source_ref.source_kind.clone(),
			"source_id": source_ref.source_id,
			"repair_guidance": repair_guidance_for_finding_type("stale_source_ref"),
		}),
	}
}

pub(in crate::knowledge) fn stale_source_finding(
	source_ref: &KnowledgePageSourceRef,
	current: &SourceSnapshot,
) -> LintDraft {
	LintDraft {
		section_id: source_ref.section_id,
		finding_type: "stale_source_ref".to_string(),
		severity: "warning".to_string(),
		source_kind: Some(current.kind),
		source_id: Some(current.id),
		message: "Knowledge page source reference snapshot is stale.".to_string(),
		details: serde_json::json!({
			"stored": {
				"status": source_ref.source_status.clone(),
				"updated_at": source_ref.source_updated_at,
				"content_hash": source_ref.source_content_hash.clone(),
			},
			"current": {
				"status": current.status.clone(),
				"updated_at": current.updated_at,
				"content_hash": current.content_hash.clone(),
			},
			"repair_guidance": repair_guidance_for_finding_type("stale_source_ref"),
		}),
	}
}

pub(in crate::knowledge) fn repair_guidance_for_finding_type(finding_type: &str) -> &'static str {
	match finding_type {
		"stale_source_ref" =>
			"Inspect the stale or missing source, then rebuild the page from current authoritative sources.",
		"unsupported_claim" =>
			"Replace the unsupported section content with source-backed text or rebuild from cited sources.",
		"missing_citation" =>
			"Rebuild the page section with explicit citations or mark the section unsupported with a reason.",
		"missing_source_ref" =>
			"Rebuild the page so each section citation is normalized into knowledge_page_source_refs.",
		"low_source_coverage" =>
			"Rebuild with all intended sources or remove uncited material before relying on this page.",
		_ => "Inspect the finding and rebuild the page after source review.",
	}
}

pub(in crate::knowledge) fn source_changed(
	source_ref: &KnowledgePageSourceRef,
	current: &SourceSnapshot,
) -> bool {
	source_ref.source_status.as_deref() != current.status.as_deref()
		|| source_ref.source_updated_at != current.updated_at
		|| source_ref.source_content_hash.as_deref() != current.content_hash.as_deref()
}
