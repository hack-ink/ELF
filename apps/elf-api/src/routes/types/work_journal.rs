use crate::routes::types::{
	Deserialize, Uuid, Value, WorkJournalEntryFamily, WritePolicy, empty_json_object,
};

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct WorkJournalEntryCreateBody {
	pub(in crate::routes) entry_id: Option<Uuid>,
	pub(in crate::routes) scope: String,
	pub(in crate::routes) session_id: String,
	pub(in crate::routes) family: WorkJournalEntryFamily,
	pub(in crate::routes) title: Option<String>,
	pub(in crate::routes) body: String,
	pub(in crate::routes) source_refs: Vec<Value>,
	pub(in crate::routes) write_policy: Option<WritePolicy>,
	#[serde(default)]
	pub(in crate::routes) explicit_next_steps: Vec<String>,
	#[serde(default)]
	pub(in crate::routes) inferred_next_steps: Vec<String>,
	#[serde(default)]
	pub(in crate::routes) rejected_options: Vec<String>,
	#[serde(default = "empty_json_object")]
	pub(in crate::routes) promotion_boundary: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct WorkJournalSessionReadbackBody {
	pub(in crate::routes) session_id: String,
	#[serde(default)]
	pub(in crate::routes) families: Vec<WorkJournalEntryFamily>,
	pub(in crate::routes) limit: Option<u32>,
}
