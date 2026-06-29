mod sources;

pub use self::sources::{
	KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeEventSource, KnowledgeNoteSource,
	KnowledgeProposalSource, KnowledgeRelationSource, KnowledgeRelationSourcesFetch,
};

use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Arguments for upserting one derived knowledge page.
pub struct KnowledgePageUpsert<'a> {
	/// Page identifier to use for a newly created page.
	pub page_id: Uuid,
	/// Tenant that owns the page.
	pub tenant_id: &'a str,
	/// Project that owns the page.
	pub project_id: &'a str,
	/// Page kind.
	pub page_kind: &'a str,
	/// Stable page key.
	pub page_key: &'a str,
	/// Page title.
	pub title: &'a str,
	/// Versioned page contract schema.
	pub contract_schema: &'a str,
	/// Page lifecycle status.
	pub status: &'a str,
	/// Canonical source snapshot hash.
	pub rebuild_source_hash: &'a str,
	/// Canonical page content hash.
	pub content_hash: &'a str,
	/// Source coverage metadata.
	pub source_coverage: &'a Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: &'a Value,
	/// Rebuild metadata.
	pub rebuild_metadata: &'a Value,
	/// Rebuild timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting one knowledge page section.
pub struct KnowledgePageSectionInsert<'a> {
	/// Section identifier.
	pub section_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Stable section key.
	pub section_key: &'a str,
	/// Section heading.
	pub heading: &'a str,
	/// Section role.
	pub role: &'a str,
	/// Section content.
	pub content: &'a str,
	/// Section display order.
	pub ordinal: i32,
	/// Section citations.
	pub citations: &'a Value,
	/// Reason the section has no citations, when intentionally unsupported.
	pub unsupported_reason: Option<&'a str>,
	/// Section content hash.
	pub content_hash: &'a str,
	/// Creation/update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting one normalized knowledge page citation.
pub struct KnowledgePageSourceRefInsert<'a> {
	/// Source-reference row identifier.
	pub ref_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Section that cites the source, if section-scoped.
	pub section_id: Option<Uuid>,
	/// Source kind.
	pub source_kind: &'a str,
	/// Authoritative source identifier.
	pub source_id: Uuid,
	/// Captured source status.
	pub source_status: Option<&'a str>,
	/// Captured source updated timestamp.
	pub source_updated_at: Option<OffsetDateTime>,
	/// Captured source content hash.
	pub source_content_hash: Option<&'a str>,
	/// Captured source snapshot.
	pub source_snapshot: &'a Value,
	/// Citation-local metadata.
	pub citation_metadata: &'a Value,
	/// Creation timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting one knowledge page lint finding.
pub struct KnowledgePageLintFindingInsert<'a> {
	/// Lint finding identifier.
	pub finding_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Section associated with the finding, when available.
	pub section_id: Option<Uuid>,
	/// Finding type.
	pub finding_type: &'a str,
	/// Finding severity.
	pub severity: &'a str,
	/// Source kind associated with the finding, when available.
	pub source_kind: Option<&'a str>,
	/// Source identifier associated with the finding, when available.
	pub source_id: Option<Uuid>,
	/// Human-readable finding message.
	pub message: &'a str,
	/// Structured finding details.
	pub details: &'a Value,
	/// Creation timestamp.
	pub now: OffsetDateTime,
}

/// Searchable knowledge page section row with page and lint metadata.
#[derive(Debug, FromRow)]
pub struct KnowledgePageSearchRow {
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
	/// Source coverage metadata.
	pub source_coverage: Value,
	/// Rebuild metadata.
	pub rebuild_metadata: Value,
	/// Page update timestamp.
	pub page_updated_at: OffsetDateTime,
	/// Page rebuild timestamp.
	pub rebuilt_at: OffsetDateTime,
	/// Section identifier.
	pub section_id: Uuid,
	/// Stable section key.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section role.
	pub role: String,
	/// Section content.
	pub content: String,
	/// Section display order.
	pub ordinal: i32,
	/// Section citations.
	pub citations: Value,
	/// Reason the section is unsupported, when present.
	pub unsupported_reason: Option<String>,
	/// Number of error lint findings for the page.
	pub lint_error_count: i64,
	/// Number of warning lint findings for the page.
	pub lint_warning_count: i64,
	/// Number of info lint findings for the page.
	pub lint_info_count: i64,
	/// Number of normalized source refs for this section.
	pub section_source_ref_count: i64,
}
