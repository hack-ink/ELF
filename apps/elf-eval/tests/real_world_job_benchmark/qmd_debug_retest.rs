use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn qmd_debug_ergonomics_dreaming_retest_report_preserves_qmd_edge() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::qmd_debug_ergonomics_dreaming_retest_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::qmd_debug_ergonomics_dreaming_retest_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_qmd_debug_retest_summary(&report)?;
	assert_qmd_debug_retest_command_and_adapters(&report)?;
	assert_qmd_debug_retest_scenarios(&report)?;
	assert_qmd_debug_retest_boundaries(&report)?;
	assert_qmd_debug_retest_markdown_and_indexes(&markdown, &benchmarking_index, &readme);

	Ok(())
}

fn assert_qmd_debug_retest_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.qmd_debug_ergonomics_dreaming_retest_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-982"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("unchanged_with_live_operator_debug_confirmation")
	);
	assert_eq!(
		report.pointer("/summary/debug_ergonomics_edge").and_then(Value::as_str),
		Some("qmd_default_top10_and_short_cli_replay_preserved")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/improved_scenario_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/regressed_scenario_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/unchanged_scenario_count").and_then(Value::as_u64),
		Some(6)
	);
	assert!(support::array_contains_str(
		report,
		"/summary/unsupported_claims_rejected",
		"qmd's live operator-debug wrong_result rows do not erase qmd's default top-k and short CLI replay edge."
	)?);

	Ok(())
}

fn assert_qmd_debug_retest_command_and_adapters(report: &Value) -> Result<()> {
	let command = support::find_by_field(
		support::array_at(report, "/commands")?,
		"/command",
		"cargo make real-world-job-operator-ux-live-adapters",
	)?;

	assert_eq!(command.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		command.pointer("/summary/schema").and_then(Value::as_str),
		Some("elf.real_world_operator_debug_live_adapter_sweep/v1")
	);

	let adapters = support::array_at(report, "/adapter_summaries")?;
	let elf = support::find_by_field(adapters, "/adapter_id", "elf_operator_debug_live")?;
	let qmd = support::find_by_field(adapters, "/adapter_id", "qmd_operator_debug_live")?;

	assert_eq!(elf.pointer("/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(elf.pointer("/trace_available_count").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/replay_command_available_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(qmd.pointer("/wrong_result").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/trace_available_count").and_then(Value::as_u64), Some(0));
	assert_eq!(qmd.pointer("/trace_incomplete_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/replay_command_available_count").and_then(Value::as_u64), Some(6));

	Ok(())
}

fn assert_qmd_debug_retest_scenarios(report: &Value) -> Result<()> {
	let scenarios = support::array_at(report, "/scenario_retests")?;
	let top10 =
		support::find_by_field(scenarios, "/scenario_id", "qmd_default_top10_candidate_artifact")?;
	let replay = support::find_by_field(scenarios, "/scenario_id", "qmd_short_cli_replay")?;
	let trace =
		support::find_by_field(scenarios, "/scenario_id", "elf_operator_debug_trace_hydration")?;
	let candidate = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let expansion =
		support::find_by_field(scenarios, "/scenario_id", "query_expansion_attribution")?;
	let fusion = support::find_by_field(scenarios, "/scenario_id", "fusion_attribution")?;
	let rerank = support::find_by_field(scenarios, "/scenario_id", "rerank_attribution")?;

	assert_eq!(scenarios.len(), 10);
	assert_eq!(top10.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(top10.pointer("/current_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(replay.pointer("/current_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(
		trace.pointer("/current_counts/elf_trace_available").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		trace.pointer("/current_counts/qmd_trace_available").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		candidate
			.pointer("/current_counts/qmd_intermediate_stage_visible_jobs")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert!(support::array_contains_str(
		candidate,
		"/typed_non_pass_states",
		"retrieved_but_dropped"
	)?);
	assert_eq!(expansion.pointer("/judgment").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(fusion.pointer("/judgment").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(rerank.pointer("/judgment").and_then(Value::as_str), Some("non_goal"));

	Ok(())
}

fn assert_qmd_debug_retest_boundaries(report: &Value) -> Result<()> {
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/allowed",
		"qmd's default local-debug edge remains: top-10 candidate rows plus short CLI replay."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF broadly beats qmd from this retest."
	)?);
	assert!(support::array_contains_str(
		report,
		"/next_optimization_direction/required_fields",
		"fusion_rank_deltas"
	)?);

	Ok(())
}

fn assert_qmd_debug_retest_markdown_and_indexes(
	markdown: &str,
	benchmarking_index: &str,
	readme: &str,
) {
	assert!(markdown.contains("The qmd debug-ergonomics outcome is unchanged"));
	assert!(markdown.contains("ELF 6 pass/0 wrong_result; qmd 0 pass/6 wrong_result"));
	assert!(
		markdown.contains("Do not treat qmd's 0 pass/6 wrong_result live operator-debug slice")
	);
	assert!(markdown.contains("Immediate top-k rows with source id"));
	assert!(
		benchmarking_index.contains("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md")
	);
	assert!(readme.contains("qmd Debug-Ergonomics Dreaming Retest Report - June 19, 2026"));
	assert!(readme.contains("Temporal and Trajectory Adapter Coverage Report - June 23, 2026"));
	assert!(readme.contains("Latest real-world benchmark report: June 27, 2026"));
	assert!(readme.contains("keeps the qmd edge unchanged"));
}
