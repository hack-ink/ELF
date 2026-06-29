use crate::routes::{Deserialize, Map, Serialize, Uuid, Value};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in super::super) enum SearchMode {
	QuickFind,
	PlannedSearch,
}

#[derive(Clone, Debug, Deserialize)]
pub(in super::super) struct EntityMemoryQuery {
	pub(in super::super) entity_id: Option<Uuid>,
	pub(in super::super) entity_surface: Option<String>,
}

pub(in super::super) fn empty_json_object() -> Value {
	Value::Object(Map::new())
}
