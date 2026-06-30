use crate::recall_debug::{SearchTrace, SearchTrajectoryStage, Value};

pub(in crate::recall_debug::replay) fn compact_replay_controls(trace: &SearchTrace) -> Value {
	serde_json::json!({
		"top_k": trace.top_k,
		"candidate_count": trace.candidate_count,
		"expansion_mode": trace.expansion_mode,
		"expanded_query_count": trace.expanded_queries.len(),
		"expanded_queries": trace.expanded_queries,
		"allowed_scopes": trace.allowed_scopes,
		"search": compact_pointer(&trace.config_snapshot, "/search"),
		"ranking": {
			"policy_id": compact_pointer(&trace.config_snapshot, "/ranking/policy_id"),
			"blend": compact_pointer(&trace.config_snapshot, "/ranking/blend"),
			"diversity": compact_pointer(&trace.config_snapshot, "/ranking/diversity"),
			"retrieval_sources": compact_pointer(&trace.config_snapshot, "/ranking/retrieval_sources"),
			"override": compact_pointer(&trace.config_snapshot, "/ranking/override"),
		},
	})
}

pub(in crate::recall_debug::replay) fn compact_stage_movement(
	stages: &[SearchTrajectoryStage],
) -> Vec<Value> {
	stages
		.iter()
		.map(|stage| {
			serde_json::json!({
				"stage_order": stage.stage_order,
				"stage_name": stage.stage_name,
				"item_count": stage.items.len(),
				"stats": compact_pointer(&stage.stage_payload, "/stats"),
				"decisions": compact_pointer(&stage.stage_payload, "/decisions"),
				"filter_impact": compact_pointer(&stage.stage_payload, "/filter_impact"),
			})
		})
		.collect()
}

fn compact_pointer(value: &Value, pointer: &str) -> Value {
	value.pointer(pointer).cloned().unwrap_or(Value::Null)
}
