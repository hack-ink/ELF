use serde_json::Value;
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub(in crate::recall_debug) struct NoteDebugSourceRow {
	pub(in crate::recall_debug) status: String,
	pub(in crate::recall_debug) source_ref: Value,
	pub(in crate::recall_debug) updated_at: OffsetDateTime,
}
