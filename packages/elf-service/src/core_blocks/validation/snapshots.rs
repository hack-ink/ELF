use serde_json::Value;

use crate::core_blocks::rows::{CoreBlockAttachmentRow, CoreBlockRow};

pub(in crate::core_blocks) fn block_snapshot(block: &CoreBlockRow) -> Value {
	serde_json::json!({
		"block_id": block.block_id,
		"tenant_id": block.tenant_id,
		"project_id": block.project_id,
		"agent_id": block.agent_id,
		"scope": block.scope,
		"key": block.key,
		"title": block.title,
		"content": block.content,
		"source_ref": block.source_ref,
		"status": block.status,
		"created_at": block.created_at,
		"updated_at": block.updated_at,
	})
}

pub(in crate::core_blocks) fn attachment_snapshot(attachment: &CoreBlockAttachmentRow) -> Value {
	serde_json::json!({
		"attachment_id": attachment.attachment_id,
		"block_id": attachment.block_id,
		"tenant_id": attachment.tenant_id,
		"project_id": attachment.project_id,
		"agent_id": attachment.agent_id,
		"read_profile": attachment.read_profile,
		"attached_by_agent_id": attachment.attached_by_agent_id,
		"attached_at": attachment.attached_at,
		"detached_by_agent_id": attachment.detached_by_agent_id,
		"detached_at": attachment.detached_at,
	})
}
