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

/// Parameters for fetching graph relation sources for knowledge pages.
pub struct KnowledgeRelationSourcesFetch<'a> {
	/// Tenant that owns the relation sources.
	pub tenant_id: &'a str,
	/// Project that owns the relation sources.
	pub project_id: &'a str,
	/// Agent requesting source readback, when visibility should be caller-scoped.
	pub agent_id: Option<&'a str>,
	/// Scopes allowed by the caller read profile.
	pub allowed_scopes: &'a [String],
	/// Shared owner/scope grant keys readable by the caller.
	pub shared_scope_keys: &'a [String],
	/// Whether private scope is readable by the caller.
	pub private_allowed: bool,
	/// Graph fact identifiers to fetch.
	pub fact_ids: &'a [Uuid],
}

/// Authoritative note source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeNoteSource {
	/// Note identifier.
	pub note_id: Uuid,
	/// Agent that owns the note.
	pub agent_id: String,
	/// Note scope.
	pub scope: String,
	/// Note type.
	pub note_type: String,
	/// Optional note key.
	pub key: Option<String>,
	/// Note text.
	pub text: String,
	/// Note importance.
	pub importance: f32,
	/// Note confidence.
	pub confidence: f32,
	/// Note status.
	pub status: String,
	/// Note creation timestamp.
	pub created_at: OffsetDateTime,
	/// Note update timestamp.
	pub updated_at: OffsetDateTime,
	/// Optional note expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Note embedding version.
	pub embedding_version: String,
	/// Opaque note source reference.
	pub source_ref: Value,
}

/// Durable add_event audit source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeEventSource {
	/// Ingest decision identifier.
	pub decision_id: Uuid,
	/// Agent that wrote the audited event-derived note decision.
	pub agent_id: String,
	/// Scope associated with the audited decision.
	pub scope: String,
	/// Ingestion pipeline name.
	pub pipeline: String,
	/// Event-derived note type.
	pub note_type: String,
	/// Optional note key.
	pub note_key: Option<String>,
	/// Note identifier affected by the decision, when persisted.
	pub note_id: Option<Uuid>,
	/// Policy decision.
	pub policy_decision: String,
	/// Note operation.
	pub note_op: String,
	/// Optional reason code.
	pub reason_code: Option<String>,
	/// Structured audit details.
	pub details: Value,
	/// Audit timestamp.
	pub ts: OffsetDateTime,
}

/// Authoritative graph relation source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeRelationSource {
	/// Graph fact identifier.
	pub fact_id: Uuid,
	/// Agent that wrote the fact.
	pub agent_id: String,
	/// Fact scope.
	pub scope: String,
	/// Subject canonical text.
	pub subject: String,
	/// Optional subject kind.
	pub subject_kind: Option<String>,
	/// Predicate text.
	pub predicate: String,
	/// Optional object entity canonical text.
	pub object_entity: Option<String>,
	/// Optional object entity kind.
	pub object_kind: Option<String>,
	/// Optional scalar object value.
	pub object_value: Option<String>,
	/// Fact validity window start.
	pub valid_from: OffsetDateTime,
	/// Fact validity window end, when historical.
	pub valid_to: Option<OffsetDateTime>,
	/// Fact update timestamp.
	pub updated_at: OffsetDateTime,
	/// Evidence notes linked to this fact.
	pub evidence_notes: Value,
}

/// Reviewed consolidation proposal source row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeProposalSource {
	/// Consolidation proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Agent that registered the proposal.
	pub agent_id: String,
	/// Proposal kind.
	pub proposal_kind: String,
	/// Proposal apply intent.
	pub apply_intent: String,
	/// Proposal review state.
	pub review_state: String,
	/// Serialized proposal source references.
	pub source_refs: Value,
	/// Serialized proposal source snapshot.
	pub source_snapshot: Value,
	/// Serialized proposal lineage.
	pub lineage: Value,
	/// Serialized proposal diff.
	pub diff: Value,
	/// Proposal confidence.
	pub confidence: f32,
	/// Unsupported claim flags.
	pub unsupported_claim_flags: Value,
	/// Contradiction markers.
	pub contradiction_markers: Value,
	/// Staleness markers.
	pub staleness_markers: Value,
	/// Derived target reference.
	pub target_ref: Value,
	/// Proposed derived payload.
	pub proposed_payload: Value,
	/// Proposal update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Source Library document row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeDocSource {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Agent that captured the document.
	pub agent_id: String,
	/// Document scope.
	pub scope: String,
	/// Document type.
	pub doc_type: String,
	/// Document lifecycle status.
	pub status: String,
	/// Optional document title.
	pub title: Option<String>,
	/// Document source reference.
	pub source_ref: Value,
	/// Persisted document content.
	pub content: String,
	/// Persisted byte length.
	pub content_bytes: i32,
	/// Whole-document content hash.
	pub content_hash: String,
	/// Document creation timestamp.
	pub created_at: OffsetDateTime,
	/// Document update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Source Library document chunk row used by the knowledge page rebuilder.
#[derive(Debug, FromRow)]
pub struct KnowledgeDocChunkSource {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Parent document identifier.
	pub doc_id: Uuid,
	/// Agent that captured the document.
	pub agent_id: String,
	/// Document scope.
	pub scope: String,
	/// Document type.
	pub doc_type: String,
	/// Document lifecycle status.
	pub status: String,
	/// Optional document title.
	pub title: Option<String>,
	/// Document source reference.
	pub source_ref: Value,
	/// Whole-document content hash.
	pub doc_content_hash: String,
	/// Document update timestamp.
	pub doc_updated_at: OffsetDateTime,
	/// Zero-based chunk index.
	pub chunk_index: i32,
	/// Inclusive start byte offset.
	pub start_offset: i32,
	/// Exclusive end byte offset.
	pub end_offset: i32,
	/// Chunk text.
	pub chunk_text: String,
	/// Chunk content hash.
	pub chunk_hash: String,
	/// Chunk creation timestamp.
	pub chunk_created_at: OffsetDateTime,
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
