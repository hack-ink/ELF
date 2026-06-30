use crate::knowledge::api::{
	KnowledgePageLintFindingResponse, KnowledgePageResponse, KnowledgePageSummary, Serialize, Uuid,
};

/// Response returned after rebuilding a derived knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageRebuildResponse {
	/// Rebuilt page with sections, source refs, and lint findings.
	pub page: KnowledgePageResponse,
}

/// Response returned by derived knowledge page listing.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePagesListResponse {
	/// Returned pages.
	pub pages: Vec<KnowledgePageSummary>,
}

/// Response returned after linting one knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintResponse {
	/// Page identifier.
	pub page_id: Uuid,
	/// Current lint findings.
	pub findings: Vec<KnowledgePageLintFindingResponse>,
}
