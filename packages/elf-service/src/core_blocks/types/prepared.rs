use serde_json::Value;
use uuid::Uuid;

pub(in crate::core_blocks) struct PreparedGetRequest {
	pub(in crate::core_blocks) tenant_id: String,
	pub(in crate::core_blocks) project_id: String,
	pub(in crate::core_blocks) agent_id: String,
	pub(in crate::core_blocks) read_profile: String,
	pub(in crate::core_blocks) allowed_scopes: Vec<String>,
}

pub(in crate::core_blocks) struct PreparedUpsertRequest {
	pub(in crate::core_blocks) tenant_id: String,
	pub(in crate::core_blocks) project_id: String,
	pub(in crate::core_blocks) agent_id: String,
	pub(in crate::core_blocks) block_id: Option<Uuid>,
	pub(in crate::core_blocks) scope: String,
	pub(in crate::core_blocks) key: String,
	pub(in crate::core_blocks) title: String,
	pub(in crate::core_blocks) content: String,
	pub(in crate::core_blocks) source_ref: Value,
	pub(in crate::core_blocks) reason: String,
}

pub(in crate::core_blocks) struct PreparedAttachRequest {
	pub(in crate::core_blocks) tenant_id: String,
	pub(in crate::core_blocks) project_id: String,
	pub(in crate::core_blocks) agent_id: String,
	pub(in crate::core_blocks) block_id: Uuid,
	pub(in crate::core_blocks) target_agent_id: String,
	pub(in crate::core_blocks) read_profile: String,
	pub(in crate::core_blocks) allowed_scopes: Vec<String>,
	pub(in crate::core_blocks) reason: String,
}

pub(in crate::core_blocks) struct PreparedDetachRequest {
	pub(in crate::core_blocks) tenant_id: String,
	pub(in crate::core_blocks) project_id: String,
	pub(in crate::core_blocks) agent_id: String,
	pub(in crate::core_blocks) attachment_id: Uuid,
	pub(in crate::core_blocks) reason: String,
}
