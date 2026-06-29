use serde_json::Value;

use crate::{CaptureRuntimeSourceRefEvidence, LiveCaptureAction, LiveCapturePolicy};

fn capture_item(
	evidence_id: &str,
	action: LiveCaptureAction,
	source_id: Option<&str>,
	evidence_binding: Option<&str>,
	write_policy: Option<Value>,
) -> super::CorpusText {
	super::CorpusText {
		evidence_id: evidence_id.to_string(),
		text: "Public capture text.".to_string(),
		capture: LiveCapturePolicy {
			action,
			source_id: source_id.map(ToString::to_string),
			evidence_binding: evidence_binding.map(ToString::to_string),
			write_policy,
		},
	}
}

fn capture_evidence(stored: &[&str], excluded: &[&str]) -> super::CaptureMaterializationEvidence {
	super::CaptureMaterializationEvidence {
		stored_evidence_ids: stored.iter().map(|id| (*id).to_string()).collect(),
		excluded_evidence_ids: excluded.iter().map(|id| (*id).to_string()).collect(),
		source_ids: Vec::new(),
		write_policy_audit_count: 0,
		write_policy_exclusion_count: 0,
		write_policy_redaction_count: 0,
		runtime_source_refs: Vec::new(),
	}
}

#[test]
fn capture_runtime_validation_requires_returned_source_id() {
	let corpus = vec![capture_item(
		"source-a",
		super::LiveCaptureAction::Store,
		Some("capture:a"),
		None,
		None,
	)];
	let capture = capture_evidence(&["source-a"], &[]);
	let runtime = super::capture_runtime_evidence_from_source_refs([&serde_json::json!({
		"evidence_id": "source-a",
		"capture_action": "store"
	})]);
	let failure = super::validate_capture_runtime_evidence(
		"capture_integration",
		&corpus,
		&capture,
		&runtime,
	)
	.expect("missing runtime source_id should fail capture validation");

	assert!(failure.contains("did not return expected source_id capture:a"));
}

#[test]
fn capture_runtime_validation_rejects_returned_excluded_evidence() {
	let corpus = vec![capture_item(
		"private-trap",
		super::LiveCaptureAction::Exclude,
		Some("capture:private"),
		Some("negative_trap"),
		None,
	)];
	let capture = capture_evidence(&[], &["private-trap"]);
	let runtime = super::capture_runtime_evidence_from_source_refs([&serde_json::json!({
		"evidence_id": "private-trap",
		"source_id": "capture:private",
		"capture_action": "store"
	})]);
	let failure = super::validate_capture_runtime_evidence(
		"capture_integration",
		&corpus,
		&capture,
		&runtime,
	)
	.expect("returned excluded evidence should fail capture validation");

	assert!(failure.contains("excluded evidence private-trap was returned by live search"));
}

#[test]
fn capture_runtime_source_refs_are_written_into_generated_fixture() {
	let mut value = serde_json::json!({
		"corpus": {
			"items": [
				{
					"evidence_id": "source-a",
					"source_ref": {
						"schema": "source_ref/v1",
						"resolver": "fixture"
					}
				}
			]
		}
	});
	let mut capture = capture_evidence(&["source-a"], &[]);

	capture.runtime_source_refs.push(CaptureRuntimeSourceRefEvidence {
		evidence_id: "source-a".to_string(),
		source_ref: serde_json::json!({
			"schema": "real_world_live_adapter/v1",
			"evidence_id": "source-a",
			"source_id": "capture:a",
			"capture_action": "store",
			"evidence_binding": "source_ref"
		}),
	});

	super::apply_capture_runtime_source_refs(&mut value, &capture);

	assert_eq!(
		value.pointer("/corpus/items/0/source_ref/source_id").and_then(serde_json::Value::as_str),
		Some("capture:a")
	);
	assert_eq!(
		value
			.pointer("/corpus/items/0/source_ref/evidence_binding")
			.and_then(serde_json::Value::as_str),
		Some("source_ref")
	);
}
