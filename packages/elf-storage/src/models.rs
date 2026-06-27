//! Database row models shared across storage modules.

use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Persisted memory note row.
#[derive(Debug, FromRow)]
pub struct MemoryNote {
	/// Note identifier.
	pub note_id: Uuid,
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Agent that wrote the note.
	pub agent_id: String,
	/// Scope key for the note.
	pub scope: String,
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key for deduplication or lookup.
	pub key: Option<String>,
	/// Note body text.
	pub text: String,
	/// Importance score persisted for ranking.
	pub importance: f32,
	/// Confidence score persisted for ranking.
	pub confidence: f32,
	/// Lifecycle status for the note.
	pub status: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Optional expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Embedding version associated with the stored note.
	pub embedding_version: String,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Search hit counter.
	pub hit_count: i64,
	/// Timestamp of the most recent search hit.
	pub last_hit_at: Option<OffsetDateTime>,
}

/// Persisted chunk row for one memory note.
#[derive(Debug, FromRow)]
pub struct MemoryNoteChunk {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Parent note identifier.
	pub note_id: Uuid,
	/// Zero-based chunk position within the note.
	pub chunk_index: i32,
	/// Inclusive start byte offset within the original note text.
	pub start_offset: i32,
	/// Exclusive end byte offset within the original note text.
	pub end_offset: i32,
	/// Chunk text.
	pub text: String,
	/// Embedding version associated with the chunk.
	pub embedding_version: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted embedding row for one note chunk.
#[derive(Debug, FromRow)]
pub struct NoteChunkEmbedding {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Embedding version associated with the vector.
	pub embedding_version: String,
	/// Embedding dimensionality.
	pub embedding_dim: i32,
	/// Embedding vector payload.
	pub vec: Vec<f32>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// In-memory embedding payload for a full note.
#[derive(Debug)]
pub struct NoteEmbedding {
	/// Note identifier.
	pub note_id: Uuid,
	/// Embedding version associated with the vector.
	pub embedding_version: String,
	/// Embedding dimensionality.
	pub embedding_dim: i32,
	/// Embedding vector payload.
	pub vec: Vec<f32>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted note-indexing outbox row.
#[derive(Debug, FromRow)]
pub struct IndexingOutboxEntry {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Note identifier queued for indexing.
	pub note_id: Uuid,
	/// Requested indexing operation.
	pub op: String,
	/// Embedding version the worker should use.
	pub embedding_version: String,
	/// Current outbox status.
	pub status: String,
	/// Number of attempts already made.
	pub attempts: i32,
	/// Most recent failure text, if any.
	pub last_error: Option<String>,
	/// Earliest time the job may be claimed again.
	pub available_at: OffsetDateTime,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted search-trace outbox job.
#[derive(Debug, FromRow)]
pub struct TraceOutboxJob {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Trace identifier to export.
	pub trace_id: Uuid,
	/// Serialized trace payload.
	pub payload: Value,
	/// Number of attempts already made.
	pub attempts: i32,
}

/// Persisted graph entity row.
#[derive(Debug, FromRow)]
pub struct GraphEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Tenant that owns the entity.
	pub tenant_id: String,
	/// Project that owns the entity.
	pub project_id: String,
	/// Canonical entity surface.
	pub canonical: String,
	/// Normalized canonical entity surface.
	pub canonical_norm: String,
	/// Optional entity kind.
	pub kind: Option<String>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted alias row for a graph entity.
#[derive(Debug, FromRow)]
pub struct GraphEntityAlias {
	/// Alias identifier.
	pub alias_id: Uuid,
	/// Entity identifier that owns the alias.
	pub entity_id: Uuid,
	/// Alias surface.
	pub alias: String,
	/// Normalized alias surface.
	pub alias_norm: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted graph fact row.
#[derive(Debug, FromRow)]
pub struct GraphFact {
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Tenant that owns the fact.
	pub tenant_id: String,
	/// Project that owns the fact.
	pub project_id: String,
	/// Agent that emitted the fact.
	pub agent_id: String,
	/// Scope key for the fact.
	pub scope: String,
	/// Subject entity identifier.
	pub subject_entity_id: Uuid,
	/// Predicate surface captured with the fact.
	pub predicate: String,
	/// Resolved predicate identifier, when available.
	pub predicate_id: Option<Uuid>,
	/// Object entity identifier for entity-to-entity facts.
	pub object_entity_id: Option<Uuid>,
	/// Scalar object value for entity-to-value facts.
	pub object_value: Option<String>,
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	/// End of the fact validity window, if superseded.
	pub valid_to: Option<OffsetDateTime>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Evidence link between one graph fact and one memory note.
#[derive(Debug, FromRow)]
pub struct GraphFactEvidence {
	/// Evidence row identifier.
	pub evidence_id: Uuid,
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Note identifier that supports the fact.
	pub note_id: Uuid,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted graph predicate row.
#[derive(Debug, FromRow)]
pub struct GraphPredicate {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Scope key where the predicate is visible.
	pub scope_key: String,
	/// Tenant scope, when tenant-specific.
	pub tenant_id: Option<String>,
	/// Project scope, when project-specific.
	pub project_id: Option<String>,
	/// Canonical predicate surface.
	pub canonical: String,
	/// Normalized canonical predicate surface.
	pub canonical_norm: String,
	/// Cardinality policy for the predicate.
	pub cardinality: String,
	/// Lifecycle status for the predicate.
	pub status: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted alias row for a graph predicate.
#[derive(Debug, FromRow)]
pub struct GraphPredicateAlias {
	/// Alias identifier.
	pub alias_id: Uuid,
	/// Predicate identifier that owns the alias.
	pub predicate_id: Uuid,
	/// Scope key where the alias resolves.
	pub scope_key: String,
	/// Alias surface.
	pub alias: String,
	/// Normalized alias surface.
	pub alias_norm: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted source-adjacent Work Journal entry.
#[derive(Debug, FromRow)]
pub struct WorkJournalEntry {
	/// Journal entry identifier.
	pub entry_id: Uuid,
	/// Tenant that owns the entry.
	pub tenant_id: String,
	/// Project that owns the entry.
	pub project_id: String,
	/// Agent that captured the entry.
	pub agent_id: String,
	/// Visibility scope for readback.
	pub scope: String,
	/// Stable external or session-local journal session identifier.
	pub session_id: String,
	/// Entry family discriminator.
	pub family: String,
	/// Lifecycle status for the journal entry.
	pub status: String,
	/// Optional display title.
	pub title: Option<String>,
	/// Redacted durable journal body.
	pub body: String,
	/// Source references supporting this journal entry.
	pub source_refs: Value,
	/// Explicit next steps captured from the source.
	pub explicit_next_steps: Value,
	/// Inferred next steps captured as non-authoritative hints.
	pub inferred_next_steps: Value,
	/// Options rejected during the captured work session.
	pub rejected_options: Value,
	/// Promotion boundary metadata for Memory Authority and Dreaming Review.
	pub promotion_boundary: Value,
	/// Redaction audit for durable journal text.
	pub redaction_audit: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted supersession row linking two facts.
#[derive(Debug, FromRow)]
pub struct GraphFactSupersession {
	/// Supersession identifier.
	pub supersession_id: Uuid,
	/// Tenant that owns the supersession record.
	pub tenant_id: String,
	/// Project that owns the supersession record.
	pub project_id: String,
	/// Fact identifier that was superseded.
	pub from_fact_id: Uuid,
	/// Fact identifier that replaced the prior fact.
	pub to_fact_id: Uuid,
	/// Note identifier that justified the supersession.
	pub note_id: Uuid,
	/// Time the supersession took effect.
	pub effective_at: OffsetDateTime,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted consolidation run row.
#[derive(Debug, FromRow)]
pub struct ConsolidationRun {
	/// Consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Agent that registered the run.
	pub agent_id: String,
	/// Versioned consolidation contract schema.
	pub contract_schema: String,
	/// Job kind, such as fixture, manual, or scheduled.
	pub job_kind: String,
	/// Current run status.
	pub status: String,
	/// Serialized input references.
	pub input_refs: Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: Value,
	/// Serialized run lineage.
	pub lineage: Value,
	/// Structured error payload for failed runs.
	pub error: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Completion timestamp for terminal runs.
	pub completed_at: Option<OffsetDateTime>,
}

/// Persisted consolidation proposal row.
#[derive(Debug, FromRow)]
pub struct ConsolidationProposal {
	/// Consolidation proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent that registered the proposal.
	pub agent_id: String,
	/// Versioned consolidation contract schema.
	pub contract_schema: String,
	/// Proposal kind, such as derived_note or knowledge_page.
	pub proposal_kind: String,
	/// Derived-output apply intent.
	pub apply_intent: String,
	/// Current review state.
	pub review_state: String,
	/// Serialized source references.
	pub source_refs: Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: Value,
	/// Serialized proposal lineage.
	pub lineage: Value,
	/// Serialized reviewable diff.
	pub diff: Value,
	/// Proposal confidence score.
	pub confidence: f32,
	/// Serialized unsupported-claim flags.
	pub unsupported_claim_flags: Value,
	/// Serialized contradiction markers.
	pub contradiction_markers: Value,
	/// Serialized staleness markers.
	pub staleness_markers: Value,
	/// Serialized derived target reference.
	pub target_ref: Value,
	/// Serialized proposed derived output payload.
	pub proposed_payload: Value,
	/// Agent that last reviewed the proposal.
	pub reviewer_agent_id: Option<String>,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
	/// Timestamp of the last review transition.
	pub reviewed_at: Option<OffsetDateTime>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted consolidation proposal review event row.
#[derive(Debug, FromRow)]
pub struct ConsolidationProposalReviewEvent {
	/// Review event identifier.
	pub review_id: Uuid,
	/// Reviewed proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent that performed the review action.
	pub reviewer_agent_id: String,
	/// Review action requested by the reviewer.
	pub action: String,
	/// Review state before the transition.
	pub from_review_state: String,
	/// Review state after the transition.
	pub to_review_state: String,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted consolidation worker job row.
#[derive(Debug, FromRow)]
pub struct ConsolidationRunJob {
	/// Worker job identifier.
	pub job_id: Uuid,
	/// Consolidation run to materialize.
	pub run_id: Uuid,
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Agent that registered the run.
	pub agent_id: String,
	/// Job kind, such as fixture or manual.
	pub job_kind: String,
	/// Current job status.
	pub status: String,
	/// Queued proposal payload.
	pub payload: Value,
	/// Number of attempts already made.
	pub attempts: i32,
	/// Most recent failure text, if any.
	pub last_error: Option<String>,
	/// Earliest time the job may be claimed again.
	pub available_at: OffsetDateTime,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

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

/// Persisted document row.
#[derive(Debug, FromRow)]
pub struct DocDocument {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent that ingested the document.
	pub agent_id: String,
	/// Scope key for the document.
	pub scope: String,
	/// Document type discriminator.
	pub doc_type: String,
	/// Lifecycle status for the document.
	pub status: String,
	/// Optional document title.
	pub title: Option<String>,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Full document content.
	pub content: String,
	/// Byte length of the document content.
	pub content_bytes: i32,
	/// Content hash for deduplication and change detection.
	pub content_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted chunk row for one document.
#[derive(Debug, FromRow)]
pub struct DocChunk {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Parent document identifier.
	pub doc_id: Uuid,
	/// Zero-based chunk position within the document.
	pub chunk_index: i32,
	/// Inclusive start byte offset within the original document content.
	pub start_offset: i32,
	/// Exclusive end byte offset within the original document content.
	pub end_offset: i32,
	/// Chunk text.
	pub chunk_text: String,
	/// Chunk content hash.
	pub chunk_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted embedding row for one document chunk.
#[derive(Debug, FromRow)]
pub struct DocChunkEmbedding {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Embedding version associated with the vector.
	pub embedding_version: String,
	/// Embedding dimensionality.
	pub embedding_dim: i32,
	/// Embedding vector payload.
	pub vec: Vec<f32>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted document-indexing outbox row.
#[derive(Debug, FromRow)]
pub struct DocIndexingOutboxEntry {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Document identifier queued for indexing.
	pub doc_id: Uuid,
	/// Chunk identifier queued for indexing.
	pub chunk_id: Uuid,
	/// Requested indexing operation.
	pub op: String,
	/// Embedding version the worker should use.
	pub embedding_version: String,
	/// Current outbox status.
	pub status: String,
	/// Number of attempts already made.
	pub attempts: i32,
	/// Most recent failure text, if any.
	pub last_error: Option<String>,
	/// Earliest time the job may be claimed again.
	pub available_at: OffsetDateTime,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
