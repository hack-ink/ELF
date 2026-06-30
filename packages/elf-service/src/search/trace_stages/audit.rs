use crate::search::Value;

pub(in crate::search) fn build_trace_audit(actor_id: &str, token_id: Option<&str>) -> Value {
	match token_id.map(str::trim).filter(|value| !value.is_empty()) {
		Some(token_id) => serde_json::json!({ "actor_id": actor_id, "token_id": token_id }),
		None => serde_json::json!({ "actor_id": actor_id }),
	}
}
