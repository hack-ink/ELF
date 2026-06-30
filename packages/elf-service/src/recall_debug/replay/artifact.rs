use crate::recall_debug::{
	BTreeMap, BTreeSet, NoteDebugSourceRow, SearchExplainItem, SearchTrace, SearchTrajectoryStage,
	TraceReplayCandidate, Uuid, Value,
	replay::{candidate_replay, controls, selected_context},
};

pub(in crate::recall_debug) fn memory_compact_replay_artifact(
	trace: &SearchTrace,
	stages: &[SearchTrajectoryStage],
	candidates: &[TraceReplayCandidate],
	selected_items: &[&SearchExplainItem],
	selected_candidate_keys: &BTreeSet<(Uuid, Uuid)>,
	source_refs: &BTreeMap<Uuid, NoteDebugSourceRow>,
	replay_command: &str,
) -> Value {
	serde_json::json!({
		"schema": "elf.recall_debug.compact_replay/v1",
		"trace_id": trace.trace_id,
		"query": trace.query,
		"replay_command": replay_command,
		"controls": controls::compact_replay_controls(trace),
		"stage_movement": controls::compact_stage_movement(stages),
		"candidate_replay": candidate_replay::compact_candidate_replay(candidates, selected_candidate_keys, source_refs),
		"selected_context": selected_context::compact_selected_context(selected_items, source_refs),
		"authority": {
			"source_refs_visible": true,
			"policy_reasons_visible": true,
			"raw_sql_needed": false,
		},
	})
}
