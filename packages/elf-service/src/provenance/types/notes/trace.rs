use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

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
