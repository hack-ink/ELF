use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn operator_debug_fixture_reports_trace_links_and_failure_details() -> Result<()> {
	let report = support::run_json_report_from(support::operator_debug_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(7));
	assert_eq!(
		report.pointer("/summary/operator_debug_job_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(report.pointer("/summary/raw_sql_needed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/trace_incomplete_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/operator_ux_gap_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(7));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(3)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let dropped = support::find_by_field(jobs, "/job_id", "operator-debug-dropped-evidence-001")?;
	let selected =
		support::find_by_field(jobs, "/job_id", "operator-debug-selected-not-narrated-001")?;
	let compact =
		support::find_by_field(jobs, "/job_id", "operator-debug-qmd-style-compact-replay-001")?;

	assert_eq!(dropped.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		dropped.pointer("/operator_debug/raw_sql_needed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		dropped.pointer("/operator_debug/dropped_candidate_visibility").and_then(Value::as_str),
		Some("visible in Retrieval Funnel and Replay Candidates")
	);
	assert_eq!(
		dropped.pointer("/operator_debug/viewer_url").and_then(Value::as_str),
		Some("/viewer?trace_id=11111111-1111-4111-8111-111111111111")
	);
	assert_eq!(
		dropped.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("filter.read_profile")
	);
	assert!(support::array_contains_str(
		dropped,
		"/trace_explainability/stages/1/dropped_evidence",
		"trace-dropped-expected"
	)?);
	assert!(support::array_contains_str(
		dropped,
		"/trace_explainability/stages/1/distractor_evidence",
		"trace-dropped-decoy"
	)?);
	assert!(support::array_contains_str(dropped, "/produced_evidence", "trace-dropped-expected")?);
	assert_eq!(selected.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		selected.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("selection.narration")
	);
	assert_eq!(
		selected.pointer("/operator_debug/failure_mode").and_then(Value::as_str),
		Some("selected_but_not_narrated")
	);
	assert_eq!(compact.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		compact.pointer("/operator_debug/failure_mode").and_then(Value::as_str),
		Some("qmd_style_compact_replay")
	);
	assert_eq!(
		compact.pointer("/operator_debug/replay_command_available").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		compact.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("recall_debug.compact_replay")
	);
	assert!(support::array_contains_str(
		compact,
		"/trace_explainability/stages/4/kept_evidence",
		"compact-replay-artifact"
	)?);
	assert!(support::array_contains_str(
		compact,
		"/produced_evidence",
		"qmd-short-replay-reference"
	)?);

	Ok(())
}
