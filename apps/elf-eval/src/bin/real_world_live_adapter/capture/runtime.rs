use crate::{
	CaptureMaterializationEvidence, CaptureRuntimeEvidence, CaptureRuntimeEvidenceItem,
	CaptureRuntimeSourceRefEvidence, SearchItem,
};

pub(crate) fn capture_runtime_evidence_from_search_items(
	items: &[SearchItem],
) -> CaptureRuntimeEvidence {
	let source_refs = items.iter().map(|item| &item.source_ref);

	capture_runtime_evidence_from_source_refs(source_refs)
}

pub(crate) fn capture_runtime_evidence_from_source_refs<'a>(
	source_refs: impl IntoIterator<Item = &'a serde_json::Value>,
) -> CaptureRuntimeEvidence {
	let mut runtime = CaptureRuntimeEvidence::default();

	for source_ref in source_refs {
		let Some(evidence_id) = source_ref.get("evidence_id").and_then(serde_json::Value::as_str)
		else {
			continue;
		};

		if runtime.items.iter().any(|item| item.evidence_id == evidence_id) {
			continue;
		}

		runtime.items.push(CaptureRuntimeEvidenceItem {
			evidence_id: evidence_id.to_string(),
			source_id: source_ref
				.get("source_id")
				.and_then(serde_json::Value::as_str)
				.map(ToString::to_string),
			evidence_binding: source_ref
				.get("evidence_binding")
				.and_then(serde_json::Value::as_str)
				.map(ToString::to_string),
			write_policy_applied: source_ref
				.get("write_policy_applied")
				.and_then(serde_json::Value::as_bool)
				.unwrap_or(false),
			capture_action: source_ref
				.get("capture_action")
				.and_then(serde_json::Value::as_str)
				.map(ToString::to_string),
			source_ref: source_ref.clone(),
		});
	}

	runtime
}

pub(crate) fn capture_with_runtime_source_refs(
	mut capture: CaptureMaterializationEvidence,
	runtime: &CaptureRuntimeEvidence,
) -> CaptureMaterializationEvidence {
	capture.source_ids.clear();
	capture.runtime_source_refs.clear();

	for item in &runtime.items {
		if let Some(source_id) = item.source_id.as_deref() {
			crate::push_unique(&mut capture.source_ids, source_id.to_string());
		}

		capture.runtime_source_refs.push(CaptureRuntimeSourceRefEvidence {
			evidence_id: item.evidence_id.clone(),
			source_ref: item.source_ref.clone(),
		});
	}

	capture
}
