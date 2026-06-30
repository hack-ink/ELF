use crate::{BTreeSet, LoadedJob, Value};

pub(in crate::dreaming_readback) fn dreaming_readback_scoring_evidence_ids(
	loaded: &LoadedJob,
	service_evidence_ids: &[String],
) -> Vec<String> {
	let selected = service_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let trap_ids = negative_trap_evidence_ids(loaded);
	let mut evidence_ids = Vec::new();

	for evidence in &loaded.job.required_evidence {
		if selected.contains(evidence.evidence_id.as_str())
			&& !trap_ids.contains(evidence.evidence_id.as_str())
		{
			crate::push_unique(&mut evidence_ids, evidence.evidence_id.clone());
		}
	}

	if evidence_ids.is_empty() {
		for evidence_id in service_evidence_ids {
			if !trap_ids.contains(evidence_id.as_str()) {
				crate::push_unique(&mut evidence_ids, evidence_id.clone());
			}
		}
	}

	evidence_ids
}

fn negative_trap_evidence_ids(loaded: &LoadedJob) -> BTreeSet<&str> {
	loaded
		.value
		.get("negative_traps")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter(|trap| trap.get("failure_if_used").and_then(Value::as_bool).unwrap_or(false))
		.flat_map(|trap| {
			trap.get("evidence_ids")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(Value::as_str)
		})
		.collect()
}
