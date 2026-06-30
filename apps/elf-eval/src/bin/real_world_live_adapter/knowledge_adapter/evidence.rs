use crate::{
	KnowledgeMaterializationEvidence, KnowledgePageLintResponse, KnowledgePageResponse, Value,
};

pub(crate) fn knowledge_materialization_evidence(
	page: &KnowledgePageResponse,
	lint: &KnowledgePageLintResponse,
	search_result_count: usize,
) -> KnowledgeMaterializationEvidence {
	let unsupported_claim_count =
		lint.findings.iter().filter(|finding| finding.finding_type == "unsupported_claim").count()
			+ page.sections.iter().filter(|section| section.unsupported_reason.is_some()).count();

	KnowledgeMaterializationEvidence {
		page_ids: vec![page.page.page_id],
		search_result_count,
		lint_finding_count: lint.findings.len(),
		stale_source_finding_count: lint
			.findings
			.iter()
			.filter(|finding| finding.finding_type == "stale_source_ref")
			.count(),
		unsupported_claim_count,
		citation_count: page.sections.iter().map(|section| section.citation_count).sum(),
		source_ref_count: page.source_refs.len(),
		version_diff_available: page
			.page
			.previous_version_diff
			.as_ref()
			.and_then(|diff| diff.get("available"))
			.and_then(Value::as_bool)
			.unwrap_or(false),
	}
}
