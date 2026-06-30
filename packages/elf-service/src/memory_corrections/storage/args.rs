use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

pub(in crate::memory_corrections) struct RestoreNoteArgs<'a> {
	pub(in crate::memory_corrections) actor_agent_id: &'a str,
	pub(in crate::memory_corrections) reason: &'a str,
	pub(in crate::memory_corrections) correction_source_ref: &'a Value,
	pub(in crate::memory_corrections) restore_version_id: Option<Uuid>,
	pub(in crate::memory_corrections) embedding_version: &'a str,
	pub(in crate::memory_corrections) now: OffsetDateTime,
}
