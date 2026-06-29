use super::*;

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

/// Summary DTO for one derived knowledge page.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSummary {
	/// Page identifier.
	pub page_id: Uuid,
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page kind.
	pub page_kind: String,
	/// Stable page key.
	pub page_key: String,
	/// Page title.
	pub title: String,
	/// Versioned page contract schema.
	pub contract_schema: String,
	/// Page lifecycle status.
	pub status: String,
	/// Canonical source snapshot hash.
	pub rebuild_source_hash: String,
	/// Canonical page content hash.
	pub content_hash: String,
	/// Source coverage metadata.
	pub source_coverage: Value,
	/// Rebuild metadata.
	pub rebuild_metadata: Value,
	/// Previous-version diff metadata, when present.
	pub previous_version_diff: Option<Value>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Last rebuild timestamp.
	pub rebuilt_at: OffsetDateTime,
}
impl From<KnowledgePage> for KnowledgePageSummary {
	fn from(page: KnowledgePage) -> Self {
		Self {
			page_id: page.page_id,
			tenant_id: page.tenant_id,
			project_id: page.project_id,
			page_kind: page.page_kind,
			page_key: page.page_key,
			title: page.title,
			contract_schema: page.contract_schema,
			status: page.status,
			rebuild_source_hash: page.rebuild_source_hash,
			content_hash: page.content_hash,
			source_coverage: page.source_coverage,
			previous_version_diff: previous_version_diff_from_metadata(&page.rebuild_metadata),
			rebuild_metadata: page.rebuild_metadata,
			created_at: page.created_at,
			updated_at: page.updated_at,
			rebuilt_at: page.rebuilt_at,
		}
	}
}

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

/// Readback DTO for one normalized source reference.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageSourceRefResponse {
	/// Source-reference row identifier.
	pub ref_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Citing section, when section-scoped.
	pub section_id: Option<Uuid>,
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
	/// Captured source snapshot.
	pub source_snapshot: Value,
	/// Citation-local metadata.
	pub citation_metadata: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<KnowledgePageSourceRef> for KnowledgePageSourceRefResponse {
	fn from(source_ref: KnowledgePageSourceRef) -> Self {
		Self {
			ref_id: source_ref.ref_id,
			page_id: source_ref.page_id,
			section_id: source_ref.section_id,
			source_kind: source_ref.source_kind,
			source_id: source_ref.source_id,
			source_status: source_ref.source_status,
			source_updated_at: source_ref.source_updated_at,
			source_content_hash: source_ref.source_content_hash,
			source_snapshot: source_ref.source_snapshot,
			citation_metadata: source_ref.citation_metadata,
			created_at: source_ref.created_at,
		}
	}
}

/// Readback DTO for one knowledge page lint finding.
#[derive(Clone, Debug, Serialize)]
pub struct KnowledgePageLintFindingResponse {
	/// Lint finding identifier.
	pub finding_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Associated section, when available.
	pub section_id: Option<Uuid>,
	/// Finding type.
	pub finding_type: String,
	/// Finding severity.
	pub severity: String,
	/// Source kind associated with the finding, when available.
	pub source_kind: Option<String>,
	/// Source identifier associated with the finding, when available.
	pub source_id: Option<Uuid>,
	/// Human-readable finding message.
	pub message: String,
	/// Structured finding details.
	pub details: Value,
	/// Operator guidance for repair or rebuild.
	pub repair_guidance: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<KnowledgePageLintFinding> for KnowledgePageLintFindingResponse {
	fn from(finding: KnowledgePageLintFinding) -> Self {
		let repair_guidance =
			repair_guidance_for_finding_type(finding.finding_type.as_str()).to_string();

		Self {
			finding_id: finding.finding_id,
			page_id: finding.page_id,
			section_id: finding.section_id,
			finding_type: finding.finding_type,
			severity: finding.severity,
			source_kind: finding.source_kind,
			source_id: finding.source_id,
			message: finding.message,
			repair_guidance,
			details: finding.details,
			created_at: finding.created_at,
		}
	}
}
