use crate::knowledge::api::{
	KnowledgePageSection, KnowledgePageSourceRef, OffsetDateTime, Serialize, Uuid, Value,
};

/// Readback DTO for one page section.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSectionResponse {
	/// Section identifier.
	pub section_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Stable section key.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section role.
	pub role: String,
	/// Section content.
	pub content: String,
	/// Display order.
	pub ordinal: i32,
	/// Serialized citation array.
	pub citations: Value,
	/// Reason this section is intentionally unsupported, when present.
	pub unsupported_reason: Option<String>,
	/// Count of section-local citations.
	pub citation_count: usize,
	/// Count of normalized source refs attached to this section.
	pub source_ref_count: usize,
	/// True when the section has both citations and normalized source backlinks.
	pub coverage_complete: bool,
	/// Section-local normalized source backlinks.
	pub source_backlinks: Vec<KnowledgePageSectionSourceBacklink>,
	/// Section content hash.
	pub content_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
impl From<KnowledgePageSection> for KnowledgePageSectionResponse {
	fn from(section: KnowledgePageSection) -> Self {
		Self {
			section_id: section.section_id,
			page_id: section.page_id,
			section_key: section.section_key,
			heading: section.heading,
			role: section.role,
			content: section.content,
			ordinal: section.ordinal,
			citations: section.citations,
			unsupported_reason: section.unsupported_reason,
			citation_count: 0,
			source_ref_count: 0,
			coverage_complete: false,
			source_backlinks: Vec::new(),
			content_hash: section.content_hash,
			created_at: section.created_at,
			updated_at: section.updated_at,
		}
	}
}

/// Section-local source backlink used by page readback and viewer provenance.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSectionSourceBacklink {
	/// Source kind.
	pub source_kind: String,
	/// Authoritative source identifier.
	pub source_id: Uuid,
	/// Captured source status.
	pub source_status: Option<String>,
	/// Captured source update timestamp.
	pub source_updated_at: Option<OffsetDateTime>,
	/// Captured source content hash.
	pub source_content_hash: Option<String>,
}
impl From<&KnowledgePageSourceRef> for KnowledgePageSectionSourceBacklink {
	fn from(source_ref: &KnowledgePageSourceRef) -> Self {
		Self {
			source_kind: source_ref.source_kind.clone(),
			source_id: source_ref.source_id,
			source_status: source_ref.source_status.clone(),
			source_updated_at: source_ref.source_updated_at,
			source_content_hash: source_ref.source_content_hash.clone(),
		}
	}
}
