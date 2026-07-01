mod manifest_summary_scenario;
mod manifest_summary_status;

use serde_json::Value;

pub(super) fn assert_external_adapter_manifest_summary(report: &Value) {
	assert_eq!(
		report.pointer("/external_adapters/schema").and_then(Value::as_str),
		Some("elf.real_world_external_adapter_report/v1")
	);
	assert_eq!(
		report.pointer("/external_adapters/manifest_id").and_then(Value::as_str),
		Some(
			"real-world-memory-project-adapters-2026-06-11-first-generation-continuity-source-store"
		)
	);
	assert_eq!(
		report.pointer("/external_adapters/docker_isolation/default").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/docker_isolation/host_global_installs_required")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/adapter_count").and_then(Value::as_u64),
		Some(26)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/external_project_count").and_then(Value::as_u64),
		Some(19)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/fixture_backed_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/live_baseline_only_count")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/live_real_world_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(14)
	);

	manifest_summary_status::assert_external_adapter_manifest_status_summary(report);
	manifest_summary_scenario::assert_external_adapter_manifest_scenario_summary(report);
}
