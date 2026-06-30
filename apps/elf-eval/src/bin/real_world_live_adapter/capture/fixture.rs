use crate::{CaptureMaterializationEvidence, LoadedJob};

pub(crate) fn apply_capture_runtime_source_refs(
	value: &mut serde_json::Value,
	capture: &CaptureMaterializationEvidence,
) {
	let Some(items) = value.pointer_mut("/corpus/items").and_then(serde_json::Value::as_array_mut)
	else {
		return;
	};

	for item in items {
		let Some(evidence_id) = item.get("evidence_id").and_then(serde_json::Value::as_str) else {
			continue;
		};
		let Some(source_ref) = capture
			.runtime_source_refs
			.iter()
			.find(|source_ref| source_ref.evidence_id == evidence_id)
		else {
			continue;
		};

		item["source_ref"] = source_ref.source_ref.clone();
	}
}

pub(crate) fn capture_for_job(
	loaded: &LoadedJob,
	capture: CaptureMaterializationEvidence,
) -> Option<CaptureMaterializationEvidence> {
	if loaded.job.suite == "capture_integration" { Some(capture) } else { None }
}
