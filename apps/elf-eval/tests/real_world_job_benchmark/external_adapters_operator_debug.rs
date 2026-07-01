use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_operator_debug_live_adapter_records(elf: &Value, qmd: &Value) -> Result<()> {
	assert_eq!(elf.pointer("/evidence_class").and_then(Value::as_str), Some("live_real_world"));
	assert_eq!(elf.pointer("/overall_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make real-world-job-operator-ux-live-adapters")
	);
	assert_eq!(
		elf.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("operator_debugging_ux")
	);
	assert_eq!(elf.pointer("/suites/0/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/1/capability").and_then(Value::as_str),
		Some("trace_hydration_metadata")
	);
	assert_eq!(elf.pointer("/capabilities/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("replay_command_metadata")
	);
	assert_eq!(elf.pointer("/capabilities/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("candidate_drop_visibility")
	);
	assert_eq!(elf.pointer("/capabilities/3/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/4/capability").and_then(Value::as_str),
		Some("openmemory_or_claude_mem_ui_runner")
	);
	assert_eq!(elf.pointer("/capabilities/4/status").and_then(Value::as_str), Some("not_encoded"));

	let elf_scenarios = support::array_at(elf, "/scenarios")?;
	let elf_trace =
		support::find_by_field(elf_scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let elf_replay =
		support::find_by_field(elf_scenarios, "/scenario_id", "operator_debug_replay_command")?;
	let elf_candidate = support::find_by_field(
		elf_scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let elf_repair = support::find_by_field(
		elf_scenarios,
		"/scenario_id",
		"operator_debug_repair_action_clarity",
	)?;
	let elf_selected = support::find_by_field(
		elf_scenarios,
		"/scenario_id",
		"operator_debug_selected_but_not_narrated",
	)?;

	assert_eq!(elf_scenarios.len(), 5);
	assert_eq!(elf_trace.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(elf_trace.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(elf_replay.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(elf_candidate.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(elf_repair.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(elf_selected.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));

	assert_operator_debug_qmd_adapter_record(qmd)?;

	assert!(support::array_at(elf, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("narrow operator-debug live slice"))
	}));
	assert!(support::array_at(qmd, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("narrow operator-debug live slice"))
	}));

	Ok(())
}

pub(super) fn assert_operator_debug_qmd_adapter_record(qmd: &Value) -> Result<()> {
	assert_eq!(qmd.pointer("/evidence_class").and_then(Value::as_str), Some("live_real_world"));
	assert_eq!(qmd.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("operator_debugging_ux")
	);
	assert_eq!(qmd.pointer("/suites/0/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/capabilities/1/capability").and_then(Value::as_str),
		Some("local_replay_command_metadata")
	);
	assert_eq!(qmd.pointer("/capabilities/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		qmd.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("trace_hydration_metadata")
	);
	assert_eq!(qmd.pointer("/capabilities/2/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("candidate_drop_visibility")
	);
	assert_eq!(qmd.pointer("/capabilities/3/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd.pointer("/capabilities/4/status").and_then(Value::as_str), Some("not_encoded"));

	let qmd_scenarios = support::array_at(qmd, "/scenarios")?;
	let qmd_trace =
		support::find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let qmd_replay =
		support::find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_replay_command")?;
	let qmd_candidate = support::find_by_field(
		qmd_scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let qmd_repair = support::find_by_field(
		qmd_scenarios,
		"/scenario_id",
		"operator_debug_repair_action_clarity",
	)?;
	let qmd_selected = support::find_by_field(
		qmd_scenarios,
		"/scenario_id",
		"operator_debug_selected_but_not_narrated",
	)?;

	assert_eq!(qmd_scenarios.len(), 5);
	assert_eq!(qmd_trace.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd_trace.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(qmd_replay.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(qmd_replay.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(qmd_candidate.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd_candidate.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(qmd_repair.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(qmd_repair.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(qmd_selected.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd_selected.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));

	Ok(())
}
