use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_measurement_audit_json(measurement_audit_json: &Value) -> Result<()> {
	let qmd_live = support::find_by_field(
		support::array_at(measurement_audit_json, "/live_real_world_adapters")?,
		"/adapter",
		"qmd live CLI adapter",
	)?;

	assert_eq!(qmd_live.pointer("/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(qmd_live.pointer("/wrong_result").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd_live.pointer("/expected_evidence_matched").and_then(Value::as_u64), Some(38));
	assert_eq!(qmd_live.pointer("/evidence_covered_count").and_then(Value::as_u64), Some(45));

	let memory_evolution = support::find_by_field(
		support::array_at(measurement_audit_json, "/live_suite_breakdown")?,
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

	Ok(())
}

pub(crate) fn assert_retrieval_debug_profile_json(retrieval_debug_profile: &Value) {
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
}

pub(crate) fn assert_temporal_history_json(temporal_history: &Value) -> Result<()> {
	let openmemory_command = support::find_by_field(
		support::array_at(temporal_history, "/commands")?,
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

pub(crate) fn assert_competitor_strength_matrix_json(matrix: &Value) -> Result<()> {
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
