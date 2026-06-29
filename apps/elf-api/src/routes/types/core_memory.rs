use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct CoreBlockUpsertBody {
	pub(in crate::routes) block_id: Option<Uuid>,
	pub(in crate::routes) scope: String,
	pub(in crate::routes) key: String,
	pub(in crate::routes) title: String,
	pub(in crate::routes) content: String,
	#[serde(default)]
	pub(in crate::routes) source_ref: Value,
	pub(in crate::routes) reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct CoreBlockAttachBody {
	pub(in crate::routes) target_agent_id: String,
	pub(in crate::routes) read_profile: String,
	pub(in crate::routes) reason: Option<String>,
}
