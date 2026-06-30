use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// One core block audit event.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockAuditEvent {
	/// Audit event identifier.
	pub event_id: Uuid,
	/// Block identifier affected by the event.
	pub block_id: Uuid,
	/// Attachment identifier affected by the event, when applicable.
	pub attachment_id: Option<Uuid>,
	/// Agent that performed the event.
	pub actor_agent_id: String,
	/// Event type.
	pub event_type: String,
	/// Attachment target agent, when applicable.
	pub target_agent_id: Option<String>,
	/// Attachment read profile, when applicable.
	pub read_profile: Option<String>,
	/// Optional previous state snapshot.
	pub prev_snapshot: Option<Value>,
	/// Optional new state snapshot.
	pub new_snapshot: Option<Value>,
	/// Human-readable event reason.
	pub reason: String,
	#[serde(with = "crate::time_serde")]
	/// Event timestamp.
	pub ts: OffsetDateTime,
}

pub(in crate::core_blocks) struct CoreBlockEventInput<'a> {
	pub(in crate::core_blocks) block_id: Uuid,
	pub(in crate::core_blocks) attachment_id: Option<Uuid>,
	pub(in crate::core_blocks) tenant_id: &'a str,
	pub(in crate::core_blocks) project_id: &'a str,
	pub(in crate::core_blocks) actor_agent_id: &'a str,
	pub(in crate::core_blocks) event_type: &'a str,
	pub(in crate::core_blocks) target_agent_id: Option<&'a str>,
	pub(in crate::core_blocks) read_profile: Option<&'a str>,
	pub(in crate::core_blocks) prev_snapshot: Option<Value>,
	pub(in crate::core_blocks) new_snapshot: Option<Value>,
	pub(in crate::core_blocks) reason: &'a str,
	pub(in crate::core_blocks) ts: OffsetDateTime,
}
