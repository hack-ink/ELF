use crate::knowledge::api::{
	KnowledgePageLintFindingResponse, KnowledgePageSectionResponse, KnowledgePageSourceRefResponse,
	KnowledgePageSummary, Serialize,
};

/// Full readback DTO for one derived knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageResponse {
	/// Page summary.
	pub page: KnowledgePageSummary,
	/// Page sections.
	pub sections: Vec<KnowledgePageSectionResponse>,
	/// Normalized source refs.
	pub source_refs: Vec<KnowledgePageSourceRefResponse>,
	/// Lint findings.
	pub lint_findings: Vec<KnowledgePageLintFindingResponse>,
}
