use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_elf_fixture_adapter_record(adapter: &Value) -> Result<()> {
	assert_eq!(adapter.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert!(adapter.pointer("/run/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("82 jobs across 19 suites")
			&& evidence.contains("75 pass")
			&& evidence.contains("7 blocked")
			&& evidence.contains("core_archival_memory")
			&& evidence.contains("memory_summary")
			&& evidence.contains("proactive_brief")
			&& evidence.contains("scheduled_memory")
			&& evidence.contains("context_trajectory")
	}));

	let suites = support::array_at(adapter, "/suites")?;
	let core_archival = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;
	let scheduled = support::find_by_field(suites, "/suite_id", "scheduled_memory")?;
	let context_trajectory = support::find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(core_archival.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert!(core_archival.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("core block attachment")
			&& evidence.contains("project-decision recovery")
			&& evidence.contains("archival note search")
	}));
	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(scheduled.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("4 passing source-linked task readbacks")
			&& evidence.contains("private/provider scheduler blocker")
	}));
	assert_eq!(context_trajectory.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		adapter
			.pointer("/notes/1")
			.and_then(Value::as_str)
			.is_some_and(|note| note.contains("OpenViking context-trajectory measurement gates"))
	);

	Ok(())
}

pub(super) fn assert_qmd_deep_profile_gate(adapter: &Value) {
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(adapter.pointer("/run/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(adapter.pointer("/result/status").and_then(Value::as_str), Some("not_encoded"));
}

pub(super) fn assert_qmd_live_baseline_record(adapter: &Value) {
	let result_evidence = adapter.pointer("/result/evidence").and_then(Value::as_str);
	let retrieval_evidence = adapter.pointer("/suites/0/evidence").and_then(Value::as_str);

	assert!(result_evidence.is_some_and(|evidence| {
		evidence.contains("This live_baseline_only record is same-corpus evidence only")
			&& evidence.contains("cite qmd_live_real_world for the full live real-world sweep")
			&& !evidence.contains("no real_world_job qmd adapter is encoded yet")
	}));
	assert!(retrieval_evidence.is_some_and(|evidence| {
		evidence.contains("does not execute real_world_job retrieval prompts")
			&& evidence.contains("cite qmd_live_real_world for the live retrieval adapter run")
			&& !evidence.contains("no real_world_job retrieval adapter run is encoded")
	}));
}
pub(super) fn assert_openviking_deep_profile_gate(adapter: &Value) {
	let trajectory_evidence = adapter.pointer("/capabilities/1/evidence").and_then(Value::as_str);

	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert!(trajectory_evidence.is_some_and(|evidence| {
		evidence.contains("evidence-bearing same-corpus output")
			&& evidence.contains("selected hierarchy/expansion artifacts")
			&& !evidence.contains("setup reaches runnable OpenViking APIs")
	}));
}
