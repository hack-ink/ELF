use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_trace_replay_diagnostics_json(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.trace_replay_diagnostics_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-923"));
	assert_eq!(
		support::string_array_at(report, "/outcome_terms")?,
		["win", "tie", "loss", "not_tested", "blocked", "non_goal"].map(str::to_owned)
	);
	assert_eq!(
		report.pointer("/summary/retrieval_correctness").and_then(Value::as_str),
		Some("tie")
	);
	assert_eq!(report.pointer("/summary/outcome_counts/loss").and_then(Value::as_u64), Some(2));
	assert_eq!(
		report.pointer("/summary/outcome_counts/not_tested").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(report.pointer("/summary/outcome_counts/win").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/outcome_counts/tie").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/outcome_counts/non_goal").and_then(Value::as_u64), Some(1));

	assert_trace_replay_diagnostics_scenarios(report)
}

fn assert_trace_replay_diagnostics_scenarios(report: &Value) -> Result<()> {
	let scenarios = support::array_at(report, "/scenario_outcomes")?;
	let retrieval =
		support::find_by_field(scenarios, "/scenario_id", "retrieval_correctness_guardrail")?;
	let top10 =
		support::find_by_field(scenarios, "/scenario_id", "default_top10_candidate_artifact")?;
	let replay = support::find_by_field(scenarios, "/scenario_id", "replay_command_locality")?;
	let trace_surface = support::find_by_field(
		scenarios,
		"/scenario_id",
		"trace_admin_replay_surface_availability",
	)?;
	let operator_trace =
		support::find_by_field(scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let operator_replay = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_replay_command_availability",
	)?;
	let operator_candidate = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let operator_repair =
		support::find_by_field(scenarios, "/scenario_id", "operator_debug_repair_action_clarity")?;
	let operator_selected = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_selected_but_not_narrated",
	)?;
	let expansion =
		support::find_by_field(scenarios, "/scenario_id", "query_expansion_attribution")?;
	let dense_sparse =
		support::find_by_field(scenarios, "/scenario_id", "dense_sparse_channel_attribution")?;
	let fusion = support::find_by_field(scenarios, "/scenario_id", "fusion_attribution")?;
	let rerank = support::find_by_field(scenarios, "/scenario_id", "rerank_attribution")?;
	let candidate_drop =
		support::find_by_field(scenarios, "/scenario_id", "candidate_drop_diagnostics")?;
	let selected = support::find_by_field(
		scenarios,
		"/scenario_id",
		"selected_but_not_narrated_wrong_results",
	)?;
	let tombstone =
		support::find_by_field(scenarios, "/scenario_id", "evidence_absent_tombstone_diagnostics")?;

	assert_eq!(scenarios.len(), 16);
	assert_eq!(retrieval.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(top10.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(replay.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(trace_surface.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(
		operator_trace.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(operator_trace.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(operator_trace.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(operator_replay.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_candidate.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(support::array_contains_str(
		operator_candidate,
		"/typed_non_pass_states",
		"retrieved_but_dropped"
	)?);
	assert_eq!(operator_repair.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_selected.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(support::array_contains_str(
		operator_selected,
		"/typed_non_pass_states",
		"selected_but_not_narrated"
	)?);
	assert_eq!(expansion.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(dense_sparse.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(fusion.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(rerank.pointer("/result_type").and_then(Value::as_str), Some("non_goal"));
	assert_eq!(rerank.pointer("/outcome").and_then(Value::as_str), Some("non_goal"));
	assert_eq!(candidate_drop.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert!(support::array_contains_str(
		candidate_drop,
		"/typed_non_pass_states",
		"retrieved_but_dropped"
	)?);
	assert_eq!(selected.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert!(support::array_contains_str(
		selected,
		"/typed_non_pass_states",
		"selected_but_not_narrated"
	)?);
	assert_eq!(tombstone.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(tombstone.pointer("/qmd_status").and_then(Value::as_str), Some("wrong_result"));
	assert!(support::array_contains_str(
		report,
		"/wrong_result_diagnostics/qmd_missing_evidence",
		"delete-tombstone"
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"qmd currently wins the default local-debug artifact surface: top-10 rows plus short CLI replay."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"ELF narrowly wins the live operator-debug trace hydration and candidate-drop visibility slice against qmd; qmd still ties replay-command and repair-action clarity."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"Do not claim qmd beats ELF as a memory system overall."
	)?);

	Ok(())
}
