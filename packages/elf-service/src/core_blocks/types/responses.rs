use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core_blocks::types::events::CoreBlockAuditEvent;

/// Response payload for attached core block readback.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlocksResponse {
	/// Response schema identifier.
	pub schema: String,
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for attachment lookup.
	pub project_id: String,
	/// Agent requesting attached blocks.
	pub agent_id: String,
	/// Read profile used for attachment lookup.
	pub read_profile: String,
	/// Attached core blocks visible to the caller.
	pub items: Vec<CoreBlockItem>,
}

/// One attached core memory block.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockItem {
	/// Core block identifier.
	pub block_id: Uuid,
	/// Active attachment identifier that made the block visible.
	pub attachment_id: Uuid,
	/// Tenant that owns the block.
	pub tenant_id: String,
	/// Project that owns the block.
	pub project_id: String,
	/// Agent that owns the block's scope.
	pub agent_id: String,
	/// Scope key for the block.
	pub scope: String,
	/// Stable block key.
	pub key: String,
	/// Human-readable block title.
	pub title: String,
	/// Small always-attached context payload.
	pub content: String,
	/// Structured source/provenance metadata for the block.
	pub source_ref: Value,
	/// Lifecycle status for the block.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Last block update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Attachment creation timestamp.
	pub attached_at: OffsetDateTime,
	/// Agent that created the attachment.
	pub attached_by_agent_id: String,
	/// Append-only block and attachment audit events.
	pub audit_history: Vec<CoreBlockAuditEvent>,
}

/// Response payload for core block creation or update.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockUpsertResponse {
	/// Stored block record.
	pub block: CoreBlockRecord,
}

/// Core block record returned by admin mutation APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockRecord {
	/// Core block identifier.
	pub block_id: Uuid,
	/// Tenant that owns the block.
	pub tenant_id: String,
	/// Project that owns the block.
	pub project_id: String,
	/// Agent that owns the block's scope.
	pub agent_id: String,
	/// Scope key for the block.
	pub scope: String,
	/// Stable block key.
	pub key: String,
	/// Human-readable block title.
	pub title: String,
	/// Small always-attached context payload.
	pub content: String,
	/// Structured source/provenance metadata for the block.
	pub source_ref: Value,
	/// Lifecycle status for the block.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Response payload for attaching a core block.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockAttachResponse {
	/// Attachment identifier.
	pub attachment_id: Uuid,
	/// Block identifier.
	pub block_id: Uuid,
	/// Target agent for the attachment.
	pub target_agent_id: String,
	/// Exact read profile for the attachment.
	pub read_profile: String,
	/// Agent that created the attachment.
	pub attached_by_agent_id: String,
	#[serde(with = "crate::time_serde")]
	/// Attachment timestamp.
	pub attached_at: OffsetDateTime,
}

/// Response payload for detaching a core block.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockDetachResponse {
	/// Attachment identifier.
	pub attachment_id: Uuid,
	/// Whether an active attachment was detached.
	pub detached: bool,
}
