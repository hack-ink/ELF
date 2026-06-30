use std::fs;

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn openviking_trajectory_materialization_report_preserves_blocked_gates() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::openviking_trajectory_materialization_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::openviking_trajectory_materialization_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_openviking_trajectory_materialization_summary(&report)?;
	assert_openviking_trajectory_materialization_command(&report)?;
	assert_openviking_trajectory_materialization_scenarios(&report)?;
	assert_openviking_trajectory_materialization_boundaries(&report)?;
	assert_openviking_trajectory_materialization_markdown_and_indexes(
		&markdown,
		&benchmarking_index,
		&readme,
	);

	Ok(())
}

fn assert_openviking_trajectory_materialization_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.openviking_trajectory_materialization_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-983"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("materialized_blocked_context_trajectory_evidence")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/blockers_removed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked_scenario_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/regressed_scenario_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert!(support::array_contains_str(
		report,
		"/summary/unsupported_claims_rejected",
		"ELF does not beat OpenViking staged retrieval trajectory from fixture-only blocked rows."
	)?);

	Ok(())
}

fn assert_openviking_trajectory_materialization_command(report: &Value) -> Result<()> {
	let command = support::find_by_field(
		support::array_at(report, "/commands")?,
		"/command",
		"cargo make real-world-memory-context-trajectory",
	)?;
	let summary =
		command.pointer("/summary").ok_or_else(|| eyre::eyre!("missing command summary"))?;

	assert_eq!(command.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		command.pointer("/artifact_json").and_then(Value::as_str),
		Some("tmp/real-world-memory/context-trajectory/report.json")
	);
	assert_eq!(summary.pointer("/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(summary.pointer("/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(summary.pointer("/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(summary.pointer("/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(summary.pointer("/evidence_covered_count").and_then(Value::as_u64), Some(9));
	assert_eq!(summary.pointer("/source_ref_covered_count").and_then(Value::as_u64), Some(9));
	assert_eq!(summary.pointer("/quote_covered_count").and_then(Value::as_u64), Some(9));

	Ok(())
}

fn assert_openviking_trajectory_materialization_scenarios(report: &Value) -> Result<()> {
	let scenarios = support::array_at(report, "/scenario_materialization")?;
	let staged = support::find_by_field(
		scenarios,
		"/scenario_id",
		"openviking_staged_retrieval_trajectory",
	)?;
	let hierarchy =
		support::find_by_field(scenarios, "/scenario_id", "openviking_hierarchy_selection")?;
	let recursive = support::find_by_field(
		scenarios,
		"/scenario_id",
		"openviking_recursive_context_expansion",
	)?;

	assert_eq!(scenarios.len(), 3);

	for scenario in [staged, hierarchy, recursive] {
		assert_eq!(scenario.pointer("/previous_status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	}

	assert!(support::array_contains_str(
		staged,
		"/produced_evidence",
		"openviking-evidence-id-output-contract"
	)?);
	assert!(support::array_contains_str(
		hierarchy,
		"/produced_evidence",
		"hierarchy-selection-output-contract"
	)?);
	assert!(support::array_contains_str(
		recursive,
		"/produced_evidence",
		"recursive-expansion-output-contract"
	)?);
	assert_eq!(
		staged.pointer("/claim_boundary").and_then(Value::as_str),
		Some(
			"No ELF win, tie, or loss is allowed until both systems publish comparable stage artifacts for the same context-trajectory scenario."
		)
	);
	assert_eq!(
		hierarchy.pointer("/blocker").and_then(Value::as_str),
		Some("selected_hierarchy_nodes_and_evidence_ids_missing")
	);
	assert_eq!(
		recursive.pointer("/blocker").and_then(Value::as_str),
		Some("expansion_paths_and_same_corpus_evidence_ids_missing")
	);

	Ok(())
}

fn assert_openviking_trajectory_materialization_boundaries(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/improvement_regression_readback/improved").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/improvement_regression_readback/blocked").and_then(Value::as_u64),
		Some(3)
	);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/allowed",
		"The context-trajectory slice is now reproducible through cargo make real-world-memory-context-trajectory."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF beats OpenViking on staged retrieval trajectory."
	)?);
	assert!(support::array_contains_str(
		report,
		"/next_optimization_direction/required_fields",
		"expansion_path"
	)?);
	assert_eq!(
		report.pointer("/next_optimization_direction/non_goal").and_then(Value::as_str),
		Some(
			"No ELF product change or superiority claim is authorized by this materialization-only report."
		)
	);

	Ok(())
}

fn assert_openviking_trajectory_materialization_markdown_and_indexes(
	markdown: &str,
	benchmarking_index: &str,
	readme: &str,
) {
	assert!(markdown.contains("The OpenViking trajectory follow-up is now materialized"));
	assert!(markdown.contains("3 encoded jobs, 0 pass, 3 blocked, 9/9 evidence coverage"));
	assert!(markdown.contains("Do not claim ELF beats OpenViking on staged retrieval trajectory."));
	assert!(markdown.contains("OpenViking context-trajectory job can move from `blocked`"));
	assert!(
		benchmarking_index.contains("2026-06-19-openviking-trajectory-materialization-report.md")
	);
	assert!(readme.contains("OpenViking Trajectory Materialization Report - June 19, 2026"));
	assert!(readme.contains("cargo make real-world-memory-context-trajectory"));
	assert!(readme.contains("3 typed blockers with 9/9 evidence coverage"));
}
