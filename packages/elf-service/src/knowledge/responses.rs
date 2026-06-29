use super::*;

pub(super) fn section_response(
	section: KnowledgePageSection,
	source_refs: Vec<KnowledgePageSourceRef>,
) -> KnowledgePageSectionResponse {
	let citation_count = citation_count(&section.citations);
	let source_ref_count = source_refs.len();
	let source_backlinks =
		source_refs.iter().map(KnowledgePageSectionSourceBacklink::from).collect();

	KnowledgePageSectionResponse {
		citation_count,
		source_ref_count,
		coverage_complete: citation_count > 0 && source_ref_count > 0,
		source_backlinks,
		..KnowledgePageSectionResponse::from(section)
	}
}

pub(super) fn knowledge_page_search_item(
	row: KnowledgePageSearchRow,
	source_refs: Vec<KnowledgePageSourceRef>,
	query: &str,
) -> KnowledgePageSearchItem {
	let source_ref_count = usize::try_from(row.section_source_ref_count).unwrap_or(0);
	let citation_count = citation_count(&row.citations);
	let lint_summary = KnowledgePageLintSummary {
		error_count: row.lint_error_count,
		warning_count: row.lint_warning_count,
		info_count: row.lint_info_count,
		has_errors: row.lint_error_count > 0,
		has_warnings: row.lint_warning_count > 0,
	};
	let coverage_complete =
		row.source_coverage.get("coverage_complete").and_then(Value::as_bool).unwrap_or(false);
	let trust_state = search_trust_state(&lint_summary, coverage_complete, &row);
	let repair_guidance = search_repair_guidance(&trust_state);
	let previous_version_diff = previous_version_diff_from_metadata(&row.rebuild_metadata);

	KnowledgePageSearchItem {
		result_kind: "knowledge_page_section".to_string(),
		page_id: row.page_id,
		page_kind: row.page_kind,
		page_key: row.page_key,
		title: row.title,
		status: row.status,
		section_id: row.section_id,
		section_key: row.section_key,
		heading: row.heading,
		role: row.role,
		snippet: snippet_for_query(row.content.as_str(), query, SEARCH_SNIPPET_CHARS),
		citations: sanitize_search_citations(row.citations),
		citation_count,
		source_ref_count,
		source_refs: source_refs.into_iter().map(search_source_ref_response).collect(),
		source_coverage: row.source_coverage,
		rebuild_metadata: row.rebuild_metadata,
		previous_version_diff,
		lint_summary,
		trust_state,
		derived_notice:
				"Derived knowledge page snippet. Verify cited source documents, spans, memory notes, events, relations, or proposals before treating it as authoritative."
					.to_string(),
		repair_guidance,
		updated_at: row.page_updated_at,
		rebuilt_at: row.rebuilt_at,
	}
}

pub(super) fn search_source_ref_response(
	source_ref: KnowledgePageSourceRef,
) -> KnowledgePageSourceRefResponse {
	let mut response = KnowledgePageSourceRefResponse::from(source_ref);

	if response.source_kind == KnowledgeSourceKind::Proposal.as_str() {
		response.source_snapshot = sanitize_proposal_snapshot(&response.source_snapshot);
	}

	response
}

pub(super) fn sanitize_search_citations(citations: Value) -> Value {
	let Value::Array(citations) = citations else {
		return citations;
	};

	Value::Array(citations.into_iter().map(sanitize_search_citation).collect())
}

pub(super) fn sanitize_search_citation(mut citation: Value) -> Value {
	let is_proposal = citation
		.get("source_kind")
		.and_then(Value::as_str)
		.is_some_and(|kind| kind == KnowledgeSourceKind::Proposal.as_str());

	if !is_proposal {
		return citation;
	}

	if let Some(object) = citation.as_object_mut()
		&& let Some(source_snapshot) = object.get_mut("source_snapshot")
	{
		*source_snapshot = sanitize_proposal_snapshot(source_snapshot);
	}

	citation
}

pub(super) fn search_trust_state(
	lint: &KnowledgePageLintSummary,
	coverage_complete: bool,
	row: &KnowledgePageSearchRow,
) -> String {
	if lint.has_errors {
		return "derived_error".to_string();
	}
	if lint.has_warnings || row.unsupported_reason.is_some() {
		return "derived_warning".to_string();
	}

	if !coverage_complete || row.section_source_ref_count == 0 {
		return "derived_low_coverage".to_string();
	}

	"derived_clean".to_string()
}

pub(super) fn search_repair_guidance(trust_state: &str) -> Option<String> {
	match trust_state {
		"derived_error" => Some(
			"Run knowledge page lint, inspect stale or missing source refs, then rebuild the page from current authoritative sources."
				.to_string(),
		),
		"derived_warning" => Some(
			"Inspect unsupported or stale findings before using this derived snippet; rebuild after source review."
				.to_string(),
		),
		"derived_low_coverage" => Some(
			"Rebuild with complete citations or add source-backed sections before relying on this page."
				.to_string(),
		),
		_ => None,
	}
}
