use crate::{
	CaptureMaterializationEvidence, CaptureRuntimeEvidence, CaptureRuntimeEvidenceItem,
	CaptureRuntimeSourceRefEvidence, CorpusText, LiveCaptureAction, LoadedJob, Result, SearchItem,
	eyre, serde_json,
};
use elf_domain::writegate::{self, WritePolicy};

pub(super) fn capture_runtime_evidence_from_search_items(
	items: &[SearchItem],
) -> CaptureRuntimeEvidence {
	let source_refs = items.iter().map(|item| &item.source_ref);

	capture_runtime_evidence_from_source_refs(source_refs)
}

pub(super) fn capture_runtime_evidence_from_source_refs<'a>(
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

pub(super) fn capture_with_runtime_source_refs(
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

pub(super) fn validate_capture_runtime_evidence(
	suite: &str,
	corpus: &[CorpusText],
	capture: &CaptureMaterializationEvidence,
	runtime: &CaptureRuntimeEvidence,
) -> Option<String> {
	if suite != "capture_integration" {
		return None;
	}

	let mut failures = Vec::new();
	let mut expected_redactions = 0_usize;
	let mut expected_exclusions = 0_usize;

	for item in corpus {
		match item.capture.action {
			LiveCaptureAction::Exclude => {
				if runtime.item_for(item.evidence_id.as_str()).is_some() {
					failures.push(format!(
						"excluded evidence {} was returned by live search",
						item.evidence_id
					));
				}
				if capture.stored_evidence_ids.iter().any(|id| id == &item.evidence_id) {
					failures.push(format!(
						"excluded evidence {} was stored by live ingestion",
						item.evidence_id
					));
				}
				if !capture.excluded_evidence_ids.iter().any(|id| id == &item.evidence_id) {
					failures.push(format!(
						"excluded evidence {} was not recorded as excluded",
						item.evidence_id
					));
				}
			},
			LiveCaptureAction::Store => {
				let runtime_item = runtime.item_for(item.evidence_id.as_str());

				if let Some(expected_source_id) = item.capture.source_id.as_deref() {
					match runtime_item.and_then(|observed| observed.source_id.as_deref()) {
						Some(observed) if observed == expected_source_id => {},
						Some(observed) => failures.push(format!(
							"evidence {} returned source_id {observed}, expected {expected_source_id}",
							item.evidence_id
						)),
						None => failures.push(format!(
							"evidence {} did not return expected source_id {expected_source_id}",
							item.evidence_id
						)),
					}
				}
				if let Some(expected_binding) = item.capture.evidence_binding.as_deref() {
					match runtime_item.and_then(|observed| observed.evidence_binding.as_deref()) {
						Some(observed) if observed == expected_binding => {},
						Some(observed) => failures.push(format!(
							"evidence {} returned evidence_binding {observed}, expected {expected_binding}",
							item.evidence_id
						)),
						None => failures.push(format!(
							"evidence {} did not return expected evidence_binding {expected_binding}",
							item.evidence_id
						)),
					}
				}
				if let Some(policy_value) = &item.capture.write_policy {
					match write_policy_from_value(policy_value, item.evidence_id.as_str()) {
						Ok(policy) => {
							expected_exclusions += policy.exclusions.len();
							expected_redactions += policy.redactions.len();
						},
						Err(err) => failures.push(err.to_string()),
					}

					if !runtime_item.is_some_and(|observed| observed.write_policy_applied) {
						failures.push(format!(
							"evidence {} did not return write_policy_applied=true",
							item.evidence_id
						));
					}
				}
				if let Some(observed) =
					runtime_item.and_then(|observed| observed.capture_action.as_deref())
					&& observed != capture_action_str(item.capture.action)
				{
					failures.push(format!(
						"evidence {} returned capture_action {observed}, expected {}",
						item.evidence_id,
						capture_action_str(item.capture.action)
					));
				}
			},
		}
	}

	if capture.write_policy_exclusion_count < expected_exclusions {
		failures.push(format!(
			"write-policy exclusion count {} was below expected {expected_exclusions}",
			capture.write_policy_exclusion_count
		));
	}
	if capture.write_policy_redaction_count < expected_redactions {
		failures.push(format!(
			"write-policy redaction count {} was below expected {expected_redactions}",
			capture.write_policy_redaction_count
		));
	}
	if expected_exclusions + expected_redactions > 0 && capture.write_policy_audit_count == 0 {
		failures
			.push("write-policy audit count was zero despite expected policy effects".to_string());
	}
	if failures.is_empty() {
		None
	} else {
		Some(format!("Capture runtime validation failed: {}", failures.join("; ")))
	}
}

pub(super) fn elf_stored_corpus_texts(corpus: &[CorpusText]) -> Result<Vec<CorpusText>> {
	let mut stored = Vec::new();

	for item in corpus {
		if item.capture.action == LiveCaptureAction::Exclude {
			continue;
		}

		stored.push(CorpusText {
			evidence_id: item.evidence_id.clone(),
			text: transformed_capture_text(item)?.trim().to_string(),
			capture: item.capture.clone(),
		});
	}

	Ok(stored)
}

pub(super) fn write_policy_from_value(
	value: &serde_json::Value,
	evidence_id: &str,
) -> Result<WritePolicy> {
	serde_json::from_value::<WritePolicy>(value.clone()).map_err(|err| {
		eyre::eyre!("Failed to parse write_policy for evidence {evidence_id}: {err}")
	})
}

pub(super) fn apply_capture_runtime_source_refs(
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

pub(super) fn capture_for_job(
	loaded: &LoadedJob,
	capture: CaptureMaterializationEvidence,
) -> Option<CaptureMaterializationEvidence> {
	if loaded.job.suite == "capture_integration" { Some(capture) } else { None }
}

pub(super) fn capture_action_str(action: LiveCaptureAction) -> &'static str {
	match action {
		LiveCaptureAction::Store => "store",
		LiveCaptureAction::Exclude => "exclude",
	}
}

fn transformed_capture_text(item: &CorpusText) -> Result<String> {
	let Some(policy_value) = &item.capture.write_policy else {
		return Ok(item.text.clone());
	};
	let policy = write_policy_from_value(policy_value, item.evidence_id.as_str())?;
	let result =
		writegate::apply_write_policy(item.text.as_str(), Some(&policy)).map_err(|err| {
			eyre::eyre!("Invalid write_policy for evidence {}: {err:?}", item.evidence_id)
		})?;

	Ok(result.transformed)
}
