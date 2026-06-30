use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Request payload for attached core block readback.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlocksGetRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for attachment lookup.
	pub project_id: String,
	/// Agent requesting attached blocks.
	pub agent_id: String,
	/// Read profile whose exact attachments should be returned.
	pub read_profile: String,
}

/// Request payload for creating or updating a core block through admin APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockUpsertRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for the block.
	pub project_id: String,
	/// Agent creating or updating the block.
	pub agent_id: String,
	/// Existing block id to update. Omit to create.
	pub block_id: Option<Uuid>,
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
	/// Optional audit reason.
	pub reason: Option<String>,
}

/// Request payload for attaching a block to an agent/read-profile pair.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockAttachRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for the attachment.
	pub project_id: String,
	/// Agent creating the attachment.
	pub agent_id: String,
	/// Block to attach.
	pub block_id: Uuid,
	/// Target agent that should receive the block.
	pub target_agent_id: String,
	/// Exact read profile for the attachment.
	pub read_profile: String,
	/// Optional audit reason.
	pub reason: Option<String>,
}

/// Request payload for detaching a block attachment.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockDetachRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project context for the attachment.
	pub project_id: String,
	/// Agent detaching the block.
	pub agent_id: String,
	/// Attachment to detach.
	pub attachment_id: Uuid,
	/// Optional audit reason.
	pub reason: Option<String>,
}
