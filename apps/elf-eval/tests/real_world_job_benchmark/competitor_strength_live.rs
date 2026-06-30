use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn current_benchmark_reports_preserve_live_sweep_boundaries() -> Result<()> {
	let measurement_audit = fs::read_to_string(support::measurement_coverage_audit_path()?)?;
	let measurement_audit_json = serde_json::from_str::<Value>(&fs::read_to_string(
		support::measurement_coverage_audit_json_path()?,
	)?)?;
	let competitor_matrix = fs::read_to_string(support::competitor_strength_matrix_path()?)?;
	let competitor_matrix_json = serde_json::from_str::<Value>(&fs::read_to_string(
		support::competitor_strength_matrix_json_path()?,
	)?)?;
	let iteration_direction = fs::read_to_string(support::iteration_direction_report_path()?)?;
	let external_manifest = fs::read_to_string(support::external_adapter_manifest_path())?;
	let comparison_external_projects =
		fs::read_to_string(support::comparison_external_projects_path()?)?;
	let retrieval_debug_profile = serde_json::from_str::<Value>(&fs::read_to_string(
		support::retrieval_debug_profile_json_path()?,
	)?)?;
	let temporal_history = serde_json::from_str::<Value>(&fs::read_to_string(
		support::temporal_history_competitor_gap_json_path()?,
	)?)?;

	assert_current_report_text_boundaries(
		&measurement_audit,
		&competitor_matrix,
		&iteration_direction,
		&external_manifest,
		&comparison_external_projects,
	);

	assert!(competitor_matrix.contains("claude-mem work_resume remains `not_encoded`"));
	assert!(!competitor_matrix.contains("claude-mem `wrong_result`, OpenViking work_resume"));

	let qmd_live = support::find_by_field(
		support::array_at(&measurement_audit_json, "/live_real_world_adapters")?,
		"/adapter",
		"qmd live CLI adapter",
	)?;

	assert_eq!(qmd_live.pointer("/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(qmd_live.pointer("/wrong_result").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd_live.pointer("/expected_evidence_matched").and_then(Value::as_u64), Some(38));
	assert_eq!(qmd_live.pointer("/evidence_covered_count").and_then(Value::as_u64), Some(45));

	let memory_evolution = support::find_by_field(
		support::array_at(&measurement_audit_json, "/live_suite_breakdown")?,
		"/suite",
		"memory_evolution",
	)?;

	assert_eq!(
		memory_evolution.pointer("/elf_status_counts/wrong_result").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		memory_evolution.pointer("/qmd_status_counts/wrong_result").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		retrieval_debug_profile
			.pointer("/live_real_world_full_sweep_context/qmd/pass")
			.and_then(Value::as_u64),
		Some(17)
	);
	assert_eq!(
		retrieval_debug_profile
			.pointer("/live_real_world_full_sweep_context/qmd/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);

	assert_competitor_strength_matrix_json(&competitor_matrix_json)?;

	let openmemory_command = support::find_by_field(
		support::array_at(&temporal_history, "/commands")?,
		"/command",
		"cargo make openmemory-ui-export-readback",
	)?;

	assert!(
		openmemory_command
			.pointer("/artifact")
			.and_then(Value::as_str)
			.is_some_and(|artifact| artifact.contains("tmp/live-baseline/mem0-checks.json")
				&& artifact.contains("tmp/live-baseline/mem0-openmemory-ui-export.json"))
	);

	Ok(())
}

fn assert_current_report_text_boundaries(
	measurement_audit: &str,
	competitor_matrix: &str,
	iteration_direction: &str,
	external_manifest: &str,
	comparison_external_projects: &str,
) {
	assert!(
		measurement_audit.contains(
			"| `memory_evolution` | `6` | `pass:1`, `wrong_result:5` | `wrong_result:6` |"
		)
	);
	assert!(
		measurement_audit
			.contains("qmd live fails 6/6 jobs after missing the delete/TTL tombstone evidence")
	);
	assert!(measurement_audit.contains("Basic local smoke and local OSS history/readback pass"));
	assert!(measurement_audit.contains("claude-mem hook/viewer capture is `blocked`"));
	assert!(!measurement_audit.contains("claude-mem hook/viewer capture remains untested"));
	assert!(!measurement_audit.contains("blocked or untested"));

	assert_measurement_audit_adapter_status_counts(measurement_audit);

	assert!(
		competitor_matrix
			.contains("broader live suites remain `wrong_result`, `blocked`, or `not_encoded`")
	);
	assert!(competitor_matrix.contains(
		"Overall adapter-status counts: 4 `pass`,\n6 `wrong_result`, 1 `lifecycle_fail`, 7 `blocked`, and 5 `not_encoded`."
	));
	assert!(!competitor_matrix.contains("5 `blocked`, and 7 `not_encoded`"));
	assert!(
		competitor_matrix
			.contains("mem0/OpenMemory local OSS entity-scoped personalization now passes")
	);
	assert!(competitor_matrix.contains("scoped preference behavior is a measured tie"));
	assert!(
		!competitor_matrix.contains("mem0/OpenMemory and Letta personalization are `not_encoded`")
	);
	assert!(external_manifest.contains(
		"The record is a full-suite sweep, not a full-suite pass; wrong_result, blocked, and not_encoded states remain visible."
	));
	assert!(external_manifest.contains(
		"The qmd live real-world sweep covers the current encoded fixture corpus; expanded retrieval-debug strength suites still need their own materialized adapter run."
	));
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for scoped local OSS same-corpus retrieval")
	);
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for local same-corpus retrieval, reindex/update/delete")
	);
	assert!(iteration_direction.contains("| Jobs | `55` |"));
	assert!(iteration_direction.contains("| Encoded suites | `15` |"));
	assert!(iteration_direction.contains("| Pass | `49` |"));
	assert!(iteration_direction.contains("| Evidence coverage | `123/123` |"));
	assert!(iteration_direction.contains("| Expected evidence recall | `115/115` |"));

	for stale_phrase in [
		"same live sweep shape as ELF",
		"ELF and qmd live fail 5/6 jobs",
		"both systems currently fail 5/6 live memory-evolution jobs",
		"wrong_result, incomplete, blocked, and not_encoded states remain visible",
		"broader live suites remain `wrong_result`, `incomplete`, or `not_encoded`",
		"The qmd live real-world slice covers representative jobs only",
		"| Jobs | `40` |",
		"| Encoded suites | `11` |",
		"| Jobs | `50` |",
		"| Encoded suites | `14` |",
		"| Pass | `38` |",
		"| Pass | `45` |",
		"| Evidence coverage | `115/115` |",
		"| Expected evidence recall | `107/107` |",
		"history/UI/hosted/graph behavior remains",
		"current local adapter is incomplete/wrong-result",
		"current adapter is incomplete/invalid-result",
	] {
		assert!(!measurement_audit.contains(stale_phrase));
		assert!(!competitor_matrix.contains(stale_phrase));
		assert!(!iteration_direction.contains(stale_phrase));
		assert!(!external_manifest.contains(stale_phrase));
		assert!(!comparison_external_projects.contains(stale_phrase));
	}
}

fn assert_competitor_strength_matrix_json(matrix: &Value) -> Result<()> {
	let projects = support::array_at(matrix, "/project_matrix")?;
	let scenarios = support::array_at(matrix, "/scenario_matrix")?;

	assert_competitor_strength_matrix_manifest_counts(matrix);
	assert_competitor_strength_matrix_project_json(projects)?;
	assert_competitor_strength_matrix_scenario_json(scenarios)?;

	Ok(())
}

fn assert_competitor_strength_matrix_project_json(projects: &[Value]) -> Result<()> {
	let qmd = support::find_by_field(projects, "/project", "qmd")?;
	let mem0 = support::find_by_field(projects, "/project", "mem0/OpenMemory")?;
	let claude_mem = support::find_by_field(projects, "/project", "claude-mem")?;
	let openviking = support::find_by_field(projects, "/project", "OpenViking")?;

	assert_eq!(
		qmd.pointer("/current_evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(qmd.pointer("/measured_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert!(qmd.pointer("/benchmark_before_claim").and_then(Value::as_str).is_some_and(|claim| {
		claim.contains("Keep qmd deep retrieval/debug profiling separate")
			&& claim.contains("narrow operator-debug live slice")
	}));
	assert!(
		qmd.pointer("/borrow_if_stronger")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("transparent local knobs"))
	);
	assert_eq!(mem0.pointer("/measured_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		mem0.pointer("/unsupported_or_blocked_status/typed_reason").and_then(Value::as_str),
		Some("openmemory_export_helper_setup_blocked")
	);
	assert!(
		mem0.pointer("/benchmark_before_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("OpenMemory product app import/export"))
	);
	assert!(
		claude_mem
			.pointer("/unsupported_or_blocked_status/details")
			.and_then(Value::as_str)
			.is_some_and(|details| details.contains("rerun/inspection targets")
				&& details.contains("tmp/live-baseline/claude-mem-checks.json"))
	);
	assert_eq!(
		openviking.pointer("/current_evidence_class").and_then(Value::as_str),
		Some("live_baseline_only")
	);
	assert_eq!(
		openviking.pointer("/measured_status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		openviking.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("blocked")
	);
	assert!(
		openviking
			.pointer("/unsupported_or_blocked_status/details")
			.and_then(Value::as_str)
			.is_some_and(|details| details.contains("encoded as blocked fixtures"))
	);
	assert!(
		openviking
			.pointer("/benchmark_before_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("evidence-bearing same-corpus output pass"))
	);

	Ok(())
}

fn assert_competitor_strength_matrix_scenario_json(scenarios: &[Value]) -> Result<()> {
	let retrieval_debug = support::find_by_field(scenarios, "/scenario_id", "retrieval_debug")?;
	let work_resume = support::find_by_field(scenarios, "/scenario_id", "work_resume")?;
	let operator_debug = support::find_by_field(scenarios, "/scenario_id", "operator_debugging")?;
	let context_trajectory =
		support::find_by_field(scenarios, "/scenario_id", "context_trajectory")?;
	let consolidation = support::find_by_field(scenarios, "/scenario_id", "consolidation")?;

	assert!(
		retrieval_debug
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("Measured tie on encoded retrieval answers"))
	);
	assert!(retrieval_debug.pointer("/current_state").and_then(Value::as_str).is_some_and(
		|state| state.contains("qmd remains stronger on local debug ergonomics not fully scored")
	));
	assert!(
		work_resume
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("claude-mem work_resume remains not_encoded")
				&& !claim.contains("claude-mem is wrong_result"))
	);
	assert!(
		operator_debug
			.pointer("/current_elf_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("narrow live_real_world operator-debug slice"))
	);
	assert!(
		operator_debug
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd now has a narrow live_real_world"))
	);
	assert!(
		operator_debug
			.pointer("/next_measurement")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("OpenMemory and claude-mem UI/export"))
	);
	assert!(
		consolidation
			.pointer("/current_elf_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("XY-934 adds live_real_world")
				&& claim.contains("zero source mutations"))
	);
	assert!(
		consolidation
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd remains not_encoded")
				&& claim.contains("product references only"))
	);

	let personalization = support::find_by_field(scenarios, "/scenario_id", "personalization")?;

	assert_personalization_matrix_record(personalization);

	assert!(
		context_trajectory
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("not a measured live winner"))
	);
	assert!(
		context_trajectory
			.pointer("/next_measurement")
			.and_then(Value::as_str)
			.is_some_and(|measurement| measurement.contains("evidence-bearing retrieval pass"))
	);

	Ok(())
}

fn assert_personalization_matrix_record(personalization: &Value) {
	assert!(
		personalization
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim
				.contains("mem0/OpenMemory local OSS entity-scoped personalization now passes")
				&& claim.contains("Letta personalization is research_gate not_encoded"))
	);
	assert!(
		personalization
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("scoped personalization is a tie"))
	);
}

fn assert_competitor_strength_matrix_manifest_counts(matrix: &Value) {
	assert_eq!(
		matrix.pointer("/manifest_summary/adapter_records").and_then(Value::as_u64),
		Some(23)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/evidence_class_counts/live_real_world")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		matrix.pointer("/manifest_summary/overall_status_counts/pass").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		matrix.pointer("/manifest_summary/overall_status_counts/blocked").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/overall_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/overall_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);
}

fn assert_measurement_audit_adapter_status_counts(markdown: &str) {
	for expected in [
		"| `blocked` | `7` |",
		"| `not_encoded` | `5` |",
		"The generated JSON report emits `external_project_count: 16`",
	] {
		assert!(markdown.contains(expected), "missing measurement audit text: {expected}");
	}
	for stale in ["| `blocked` | `6` |", "| `not_encoded` | `6` |"] {
		assert!(!markdown.contains(stale), "stale measurement audit text: {stale}");
	}
}
