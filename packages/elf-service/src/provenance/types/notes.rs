use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_storage::models::MemoryNote;

use super::rows::{NoteIndexingOutboxRow, NoteIngestDecisionRow, NoteVersionRow};

/// Current note snapshot returned by provenance APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceNote {
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
	/// Optional application-defined key.
	pub key: Option<String>,
	/// Note body text.
	pub text: String,
	/// Importance score.
	pub importance: f32,
	/// Confidence score.
	pub confidence: f32,
	/// Lifecycle status.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// Optional expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Embedding version associated with the note.
	pub embedding_version: String,
	/// Search hit counter.
	pub hit_count: i64,
	#[serde(with = "crate::time_serde::option")]
	/// Timestamp of the most recent hit.
	pub last_hit_at: Option<OffsetDateTime>,
}
impl From<MemoryNote> for NoteProvenanceNote {
	fn from(note: MemoryNote) -> Self {
		Self {
			note_id: note.note_id,
			tenant_id: note.tenant_id,
			project_id: note.project_id,
			agent_id: note.agent_id,
			scope: note.scope,
			r#type: note.r#type,
			key: note.key,
			text: note.text,
			importance: note.importance,
			confidence: note.confidence,
			status: note.status,
			created_at: note.created_at,
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref,
			embedding_version: note.embedding_version,
			hit_count: note.hit_count,
			last_hit_at: note.last_hit_at,
		}
	}
}

/// One recorded ingestion decision for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceIngestDecision {
	/// Decision identifier.
	pub decision_id: Uuid,
	/// Tenant that owns the decision record.
	pub tenant_id: String,
	/// Project that owns the decision record.
	pub project_id: String,
	/// Agent that triggered the ingestion decision.
	pub agent_id: String,
	/// Scope key evaluated by the decision.
	pub scope: String,
	/// Pipeline name that produced the decision.
	pub pipeline: String,
	/// Note type discriminator under evaluation.
	pub note_type: String,
	/// Optional application-defined key under evaluation.
	pub note_key: Option<String>,
	/// Note identifier, when a note was persisted or matched.
	pub note_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Note version produced by this decision, when applicable.
	pub note_version_id: Option<Uuid>,
	/// Pre-policy base decision.
	pub base_decision: String,
	/// Final policy decision.
	pub policy_decision: String,
	/// Persistence operation that followed the decision.
	pub note_op: String,
	/// Machine-readable reason code, if any.
	pub reason_code: Option<String>,
	/// Structured diagnostic details.
	pub details: Value,
	#[serde(with = "crate::time_serde")]
	/// Decision timestamp.
	pub ts: OffsetDateTime,
}
impl From<NoteIngestDecisionRow> for NoteProvenanceIngestDecision {
	fn from(row: NoteIngestDecisionRow) -> Self {
		Self {
			decision_id: row.decision_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			scope: row.scope,
			pipeline: row.pipeline,
			note_type: row.note_type,
			note_key: row.note_key,
			note_id: row.note_id,
			note_version_id: row.note_version_id,
			base_decision: row.base_decision,
			policy_decision: row.policy_decision,
			note_op: row.note_op,
			reason_code: row.reason_code,
			details: row.details,
			ts: row.ts,
		}
	}
}

/// One version-history row for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceNoteVersion {
	/// Version row identifier.
	pub version_id: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Operation recorded in the version row.
	pub op: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Snapshot before the operation, when available.
	pub prev_snapshot: Option<Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Snapshot after the operation, when available.
	pub new_snapshot: Option<Value>,
	/// Human-readable reason for the change.
	pub reason: String,
	/// Actor that performed the change.
	pub actor: String,
	#[serde(with = "crate::time_serde")]
	/// Version timestamp.
	pub ts: OffsetDateTime,
}
impl From<NoteVersionRow> for NoteProvenanceNoteVersion {
	fn from(row: NoteVersionRow) -> Self {
		Self {
			version_id: row.version_id,
			note_id: row.note_id,
			op: row.op,
			prev_snapshot: row.prev_snapshot,
			new_snapshot: row.new_snapshot,
			reason: row.reason,
			actor: row.actor,
			ts: row.ts,
		}
	}
}

/// One indexing-outbox row for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceIndexingOutbox {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Requested indexing operation.
	pub op: String,
	/// Embedding version targeted by the job.
	pub embedding_version: String,
	/// Current outbox status.
	pub status: String,
	/// Number of attempts already made.
	pub attempts: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Most recent failure text, if any.
	pub last_error: Option<String>,
	#[serde(with = "crate::time_serde")]
	/// Earliest time the job may be claimed again.
	pub available_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
impl From<NoteIndexingOutboxRow> for NoteProvenanceIndexingOutbox {
	fn from(row: NoteIndexingOutboxRow) -> Self {
		Self {
			outbox_id: row.outbox_id,
			note_id: row.note_id,
			op: row.op,
			embedding_version: row.embedding_version,
			status: row.status,
			attempts: row.attempts,
			last_error: row.last_error,
			available_at: row.available_at,
			created_at: row.created_at,
			updated_at: row.updated_at,
		}
	}
}

/// Recent search trace that referenced the note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceRecentTrace {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent that ran the search.
	pub agent_id: String,
	/// Read profile used for the trace.
	pub read_profile: String,
	/// Search query text.
	pub query: String,
	#[serde(with = "crate::time_serde")]
	/// Trace creation timestamp.
	pub created_at: OffsetDateTime,
}
