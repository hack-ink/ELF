use std::collections::HashMap;

use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core_blocks::types::{CoreBlockAuditEvent, CoreBlockItem, CoreBlockRecord};

#[derive(Clone, Debug, FromRow)]
pub(in crate::core_blocks) struct CoreBlockRow {
	pub(in crate::core_blocks) block_id: Uuid,
	pub(in crate::core_blocks) tenant_id: String,
	pub(in crate::core_blocks) project_id: String,
	pub(in crate::core_blocks) agent_id: String,
	pub(in crate::core_blocks) scope: String,
	pub(in crate::core_blocks) key: String,
	pub(in crate::core_blocks) title: String,
	pub(in crate::core_blocks) content: String,
	pub(in crate::core_blocks) source_ref: Value,
	pub(in crate::core_blocks) status: String,
	pub(in crate::core_blocks) created_at: OffsetDateTime,
	pub(in crate::core_blocks) updated_at: OffsetDateTime,
}
impl CoreBlockRow {
	pub(in crate::core_blocks) fn into_record(self) -> CoreBlockRecord {
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
pub(in crate::core_blocks) struct CoreBlockAttachmentRow {
	pub(in crate::core_blocks) attachment_id: Uuid,
	pub(in crate::core_blocks) block_id: Uuid,
	pub(in crate::core_blocks) tenant_id: String,
	pub(in crate::core_blocks) project_id: String,
	pub(in crate::core_blocks) agent_id: String,
	pub(in crate::core_blocks) read_profile: String,
	pub(in crate::core_blocks) attached_by_agent_id: String,
	pub(in crate::core_blocks) attached_at: OffsetDateTime,
	pub(in crate::core_blocks) detached_by_agent_id: Option<String>,
	pub(in crate::core_blocks) detached_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::core_blocks) struct CoreBlockJoinedRow {
	pub(in crate::core_blocks) attachment_id: Uuid,
	pub(in crate::core_blocks) attachment_agent_id: String,
	pub(in crate::core_blocks) attached_by_agent_id: String,
	pub(in crate::core_blocks) attached_at: OffsetDateTime,
	pub(in crate::core_blocks) block_id: Uuid,
	pub(in crate::core_blocks) tenant_id: String,
	pub(in crate::core_blocks) project_id: String,
	pub(in crate::core_blocks) agent_id: String,
	pub(in crate::core_blocks) scope: String,
	pub(in crate::core_blocks) key: String,
	pub(in crate::core_blocks) title: String,
	pub(in crate::core_blocks) content: String,
	pub(in crate::core_blocks) source_ref: Value,
	pub(in crate::core_blocks) status: String,
	pub(in crate::core_blocks) created_at: OffsetDateTime,
	pub(in crate::core_blocks) updated_at: OffsetDateTime,
}
impl CoreBlockJoinedRow {
	pub(in crate::core_blocks) fn into_item(
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
pub(in crate::core_blocks) struct CoreBlockEventRow {
	pub(in crate::core_blocks) event_id: Uuid,
	pub(in crate::core_blocks) block_id: Uuid,
	pub(in crate::core_blocks) attachment_id: Option<Uuid>,
	pub(in crate::core_blocks) actor_agent_id: String,
	pub(in crate::core_blocks) event_type: String,
	pub(in crate::core_blocks) target_agent_id: Option<String>,
	pub(in crate::core_blocks) read_profile: Option<String>,
	pub(in crate::core_blocks) prev_snapshot: Option<Value>,
	pub(in crate::core_blocks) new_snapshot: Option<Value>,
	pub(in crate::core_blocks) reason: String,
	pub(in crate::core_blocks) ts: OffsetDateTime,
}
