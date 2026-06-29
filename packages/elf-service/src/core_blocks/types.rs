use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Core memory blocks response schema identifier.
pub const ELF_CORE_MEMORY_BLOCKS_SCHEMA_V1: &str = "elf.core_memory_blocks/v1";

pub(super) const MAX_CORE_BLOCK_CONTENT_CHARS: usize = 2_000;

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

/// Response payload for detaching a core block.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreBlockDetachResponse {
	/// Attachment identifier.
	pub attachment_id: Uuid,
	/// Whether an active attachment was detached.
	pub detached: bool,
}

#[derive(Clone, Debug, FromRow)]
pub(super) struct CoreBlockRow {
	pub(super) block_id: Uuid,
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) scope: String,
	pub(super) key: String,
	pub(super) title: String,
	pub(super) content: String,
	pub(super) source_ref: Value,
	pub(super) status: String,
	pub(super) created_at: OffsetDateTime,
	pub(super) updated_at: OffsetDateTime,
}
impl CoreBlockRow {
	pub(super) fn into_record(self) -> CoreBlockRecord {
		CoreBlockRecord {
			block_id: self.block_id,
			tenant_id: self.tenant_id,
			project_id: self.project_id,
			agent_id: self.agent_id,
			scope: self.scope,
			key: self.key,
			title: self.title,
			content: self.content,
			source_ref: self.source_ref,
			status: self.status,
			created_at: self.created_at,
			updated_at: self.updated_at,
		}
	}
}

#[derive(Clone, Debug, FromRow)]
pub(super) struct CoreBlockAttachmentRow {
	pub(super) attachment_id: Uuid,
	pub(super) block_id: Uuid,
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) read_profile: String,
	pub(super) attached_by_agent_id: String,
	pub(super) attached_at: OffsetDateTime,
	pub(super) detached_by_agent_id: Option<String>,
	pub(super) detached_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
pub(super) struct CoreBlockJoinedRow {
	pub(super) attachment_id: Uuid,
	pub(super) attachment_agent_id: String,
	pub(super) attached_by_agent_id: String,
	pub(super) attached_at: OffsetDateTime,
	pub(super) block_id: Uuid,
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) scope: String,
	pub(super) key: String,
	pub(super) title: String,
	pub(super) content: String,
	pub(super) source_ref: Value,
	pub(super) status: String,
	pub(super) created_at: OffsetDateTime,
	pub(super) updated_at: OffsetDateTime,
}
impl CoreBlockJoinedRow {
	pub(super) fn into_item(
		self,
		audit_by_block: &HashMap<Uuid, Vec<CoreBlockAuditEvent>>,
	) -> CoreBlockItem {
		let audit_history = audit_by_block.get(&self.block_id).cloned().unwrap_or_else(Vec::new);

		CoreBlockItem {
			block_id: self.block_id,
			attachment_id: self.attachment_id,
			tenant_id: self.tenant_id,
			project_id: self.project_id,
			agent_id: self.agent_id,
			scope: self.scope,
			key: self.key,
			title: self.title,
			content: self.content,
			source_ref: self.source_ref,
			status: self.status,
			updated_at: self.updated_at,
			attached_at: self.attached_at,
			attached_by_agent_id: self.attached_by_agent_id,
			audit_history,
		}
	}
}

#[derive(Clone, Debug, FromRow)]
pub(super) struct CoreBlockEventRow {
	pub(super) event_id: Uuid,
	pub(super) block_id: Uuid,
	pub(super) attachment_id: Option<Uuid>,
	pub(super) actor_agent_id: String,
	pub(super) event_type: String,
	pub(super) target_agent_id: Option<String>,
	pub(super) read_profile: Option<String>,
	pub(super) prev_snapshot: Option<Value>,
	pub(super) new_snapshot: Option<Value>,
	pub(super) reason: String,
	pub(super) ts: OffsetDateTime,
}

pub(super) struct PreparedGetRequest {
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) read_profile: String,
	pub(super) allowed_scopes: Vec<String>,
}

pub(super) struct PreparedUpsertRequest {
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) block_id: Option<Uuid>,
	pub(super) scope: String,
	pub(super) key: String,
	pub(super) title: String,
	pub(super) content: String,
	pub(super) source_ref: Value,
	pub(super) reason: String,
}

pub(super) struct PreparedAttachRequest {
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) block_id: Uuid,
	pub(super) target_agent_id: String,
	pub(super) read_profile: String,
	pub(super) allowed_scopes: Vec<String>,
	pub(super) reason: String,
}

pub(super) struct PreparedDetachRequest {
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) attachment_id: Uuid,
	pub(super) reason: String,
}

pub(super) struct CoreBlockEventInput<'a> {
	pub(super) block_id: Uuid,
	pub(super) attachment_id: Option<Uuid>,
	pub(super) tenant_id: &'a str,
	pub(super) project_id: &'a str,
	pub(super) actor_agent_id: &'a str,
	pub(super) event_type: &'a str,
	pub(super) target_agent_id: Option<&'a str>,
	pub(super) read_profile: Option<&'a str>,
	pub(super) prev_snapshot: Option<Value>,
	pub(super) new_snapshot: Option<Value>,
	pub(super) reason: &'a str,
	pub(super) ts: OffsetDateTime,
}
