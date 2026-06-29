use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

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
