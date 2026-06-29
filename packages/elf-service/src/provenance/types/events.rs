use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// One normalized memory-history event.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryHistoryEvent {
	/// Stable event identifier within its source table.
	pub event_id: String,
	/// Normalized event type.
	pub event_type: String,
	/// Subject kind for the event.
	pub subject_type: String,
	/// Inspected note identifier.
	pub note_id: Uuid,
	/// Durable source table behind the event.
	pub source_table: String,
	/// Source row identifier when available.
	pub source_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Related note version, when an ingest decision produced a version row.
	pub related_note_version_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Related ingest decision, when a version or history event was caused by ingestion.
	pub related_decision_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Related consolidation proposal, when a derived memory proposal references the note.
	pub related_proposal_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Actor that caused the event, when available.
	pub actor: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Source operation string.
	pub op: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Machine-readable reason code, when available.
	pub reason_code: Option<String>,
	/// Human-readable one-line event summary.
	pub summary: String,
	/// Source-specific event details.
	pub details: Value,
	#[serde(with = "crate::time_serde")]
	/// Event timestamp.
	pub ts: OffsetDateTime,
}
