use crate::knowledge::api::{
	KnowledgePageSourceRefResponse, OffsetDateTime, Serialize, Uuid, Value,
};

/// Response returned by derived knowledge page section search.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSearchResponse {
	/// Matching derived page snippets.
	pub items: Vec<KnowledgePageSearchItem>,
}

/// Search result for one derived knowledge page section.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSearchItem {
	/// Result type discriminator for clients that mix pages with notes.
	pub result_kind: String,
	/// Derived page identifier.
	pub page_id: Uuid,
	/// Page kind.
	pub page_kind: String,
	/// Stable page key.
	pub page_key: String,
	/// Page title.
	pub title: String,
	/// Page lifecycle status.
	pub status: String,
	/// Section identifier.
	pub section_id: Uuid,
	/// Stable section key.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section role.
	pub role: String,
	/// Bounded matching section snippet.
	pub snippet: String,
	/// Section citations for visible provenance.
	pub citations: Value,
	/// Count of section-local citations.
	pub citation_count: usize,
	/// Count of normalized source refs attached to this section.
	pub source_ref_count: usize,
	/// Section-local source refs for backlink readback.
	pub source_refs: Vec<KnowledgePageSourceRefResponse>,
	/// Page-level source coverage metadata.
	pub source_coverage: Value,
	/// Page-level rebuild metadata.
	pub rebuild_metadata: Value,
	/// Previous-version diff metadata, when present.
	pub previous_version_diff: Option<Value>,
	/// Lint summary for distinguishing clean, stale, and unsupported pages.
	pub lint_summary: KnowledgePageLintSummary,
	/// Trust state discriminator for viewer/search clients.
	pub trust_state: String,
	/// Explicit notice that the result is derived, not authoritative source truth.
	pub derived_notice: String,
	/// Repair or rebuild guidance when lint or coverage indicates risk.
	pub repair_guidance: Option<String>,
	/// Page update timestamp.
	pub updated_at: OffsetDateTime,
	/// Page rebuild timestamp.
	pub rebuilt_at: OffsetDateTime,
}

/// Aggregate lint counts for page search results.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintSummary {
	/// Error finding count.
	pub error_count: i64,
	/// Warning finding count.
	pub warning_count: i64,
	/// Info finding count.
	pub info_count: i64,
	/// True when at least one error finding exists.
	pub has_errors: bool,
	/// True when at least one warning finding exists.
	pub has_warnings: bool,
}
