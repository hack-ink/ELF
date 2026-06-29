use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

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
