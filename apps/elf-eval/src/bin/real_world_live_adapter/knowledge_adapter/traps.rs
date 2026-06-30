use crate::{LoadedJob, Value};

pub(crate) fn stale_trap_evidence_ids(loaded: &LoadedJob) -> Vec<String> {
	loaded
		.value
		.get("negative_traps")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter(|trap| {
			trap.get("type").and_then(Value::as_str) == Some("stale_fact")
				&& trap.get("failure_if_used").and_then(Value::as_bool).unwrap_or(false)
		})
		.flat_map(|trap| {
			trap.get("evidence_ids")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(Value::as_str)
				.map(ToString::to_string)
				.collect::<Vec<_>>()
		})
		.collect()
}

pub(super) fn trap_id_for_evidence(loaded: &LoadedJob, evidence_id: &str) -> Option<String> {
	loaded
		.value
		.get("negative_traps")
		.and_then(Value::as_array)?
		.iter()
		.find(|trap| {
			trap.get("evidence_ids")
				.and_then(Value::as_array)
				.is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(evidence_id)))
		})
		.and_then(|trap| trap.get("trap_id").and_then(Value::as_str))
		.map(ToString::to_string)
}
