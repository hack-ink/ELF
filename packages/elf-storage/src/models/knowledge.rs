use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Persisted derived knowledge page row.
#[derive(Debug, FromRow)]
pub struct KnowledgePage {
	/// Derived page identifier.
	pub page_id: Uuid,
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page kind, such as project, entity, concept, issue, decision, author, or timeline.
	pub page_kind: String,
	/// Stable page key within the tenant/project/kind namespace.
	pub page_key: String,
	/// Human-readable page title.
	pub title: String,
	/// Versioned knowledge page contract schema.
	pub contract_schema: String,
	/// Derived page lifecycle status.
	pub status: String,
	/// BLAKE3 hash of the canonical source snapshot.
	pub rebuild_source_hash: String,
	/// BLAKE3 hash of the canonical page payload.
	pub content_hash: String,
	/// Source coverage metadata.
	pub source_coverage: Value,
	/// Aggregate source snapshot metadata captured during rebuild.
	pub source_snapshot: Value,
	/// Rebuild metadata, including deterministic/provider information.
	pub rebuild_metadata: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Last rebuild timestamp.
	pub rebuilt_at: OffsetDateTime,
}

/// Persisted derived knowledge page section row.
#[derive(Debug, FromRow)]
pub struct KnowledgePageSection {
	/// Section identifier.
	pub section_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Stable section key within one page.
	pub section_key: String,
	/// Section heading.
	pub heading: String,
	/// Section role, such as current_truth, history, relations, or proposals.
	pub role: String,
	/// Section content.
	pub content: String,
	/// Display order within the page.
	pub ordinal: i32,
	/// Serialized citation array for this section.
	pub citations: Value,
	/// Reason a section lacks citations, when intentionally unsupported.
	pub unsupported_reason: Option<String>,
	/// BLAKE3 hash of the section content and citations.
	pub content_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted normalized citation/source reference for a knowledge page.
#[derive(Debug, FromRow)]
pub struct KnowledgePageSourceRef {
	/// Source-reference row identifier.
	pub ref_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Section that cites the source, if section-scoped.
	pub section_id: Option<Uuid>,
	/// Source kind, such as doc, doc_chunk, note, relation, proposal, or event.
	pub source_kind: String,
	/// Authoritative source identifier.
	pub source_id: Uuid,
	/// Source lifecycle status captured during rebuild.
	pub source_status: Option<String>,
	/// Source last-update timestamp captured during rebuild.
	pub source_updated_at: Option<OffsetDateTime>,
	/// Source content hash captured during rebuild.
	pub source_content_hash: Option<String>,
	/// Full source snapshot captured during rebuild.
	pub source_snapshot: Value,
	/// Citation-local metadata.
	pub citation_metadata: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted lint finding for one derived knowledge page.
#[derive(Debug, FromRow)]
pub struct KnowledgePageLintFinding {
	/// Lint finding identifier.
	pub finding_id: Uuid,
	/// Parent page identifier.
	pub page_id: Uuid,
	/// Section associated with the finding, when available.
	pub section_id: Option<Uuid>,
	/// Finding type, such as stale_source_ref or unsupported_claim.
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
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
