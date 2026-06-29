use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{NoteOp, structured_fields::StructuredFields};
use elf_domain::{
	memory_policy::MemoryPolicyDecision,
	writegate::{WritePolicy, WritePolicyAudit},
};

/// Request payload for direct note ingestion.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddNoteRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project that owns the request.
	pub project_id: String,
	/// Agent that is writing the notes.
	pub agent_id: String,
	/// Scope to apply to all notes in the batch.
	pub scope: String,
	/// Notes to validate and persist.
	pub notes: Vec<AddNoteInput>,
}

/// One note supplied to `add_note`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddNoteInput {
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key for deduplication or lookup.
	pub key: Option<String>,
	/// Note body text.
	pub text: String,
	/// Optional structured extraction payload to persist alongside the note.
	pub structured: Option<StructuredFields>,
	/// Importance score for ranking and retention.
	pub importance: f32,
	/// Confidence score for ranking and retention.
	pub confidence: f32,
	/// Optional TTL override in days.
	pub ttl_days: Option<i64>,
	#[serde(default = "default_source_ref")]
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Optional write policy applied before validation and persistence.
	pub write_policy: Option<WritePolicy>,
}

/// Per-note outcome for an `add_note` request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddNoteResult {
	/// Note identifier when one was created or updated.
	pub note_id: Option<Uuid>,
	/// Persistence operation chosen for the note.
	pub op: NoteOp,
	/// Memory-policy decision applied to the note.
	pub policy_decision: MemoryPolicyDecision,
	/// Machine-readable rejection or ignore code, if any.
	pub reason_code: Option<String>,
	/// Field path associated with a validation failure, if any.
	pub field_path: Option<String>,
	/// Write-policy audit emitted for this note, if any.
	pub write_policy_audit: Option<WritePolicyAudit>,
}

/// Response payload for direct note ingestion.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddNoteResponse {
	/// One result per requested note.
	pub results: Vec<AddNoteResult>,
}

pub(super) struct AddNoteContext<'a> {
	pub(super) tenant_id: &'a str,
	pub(super) project_id: &'a str,
	pub(super) agent_id: &'a str,
	pub(super) scope: &'a str,
	pub(super) now: OffsetDateTime,
	pub(super) embed_version: &'a str,
}

pub(super) fn default_source_ref() -> Value {
	Value::Object(Default::default())
}
