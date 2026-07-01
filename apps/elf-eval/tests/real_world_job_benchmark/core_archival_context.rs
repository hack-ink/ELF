use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn context_trajectory_fixtures_report_blocked_openviking_gates() -> Result<()> {
	let report = support::run_json_report_from(support::context_trajectory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(3)
	);

	let suites = support::array_at(&report, "/suites")?;
	let context = support::find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(context.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(context.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = support::array_at(&report, "/jobs")?;
	let staged = support::find_by_field(
		jobs,
		"/job_id",
		"context-trajectory-openviking-staged-retrieval-001",
	)?;
	let hierarchy = support::find_by_field(
		jobs,
		"/job_id",
		"context-trajectory-openviking-hierarchy-selection-001",
	)?;
	let recursive = support::find_by_field(
		jobs,
		"/job_id",
		"context-trajectory-openviking-recursive-expansion-001",
	)?;

	assert_eq!(staged.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(hierarchy.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(recursive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		staged.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("openviking.stage_artifact_gate")
	);
	assert_eq!(
		hierarchy.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("openviking.hierarchy_artifact_gate")
	);
	assert_eq!(
		recursive.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("openviking.recursive_expansion_gate")
	);

	let staged_stages = support::array_at(staged, "/trace_explainability/stages")?;
	let staged_gate =
		support::find_by_field(staged_stages, "/stage_name", "openviking.stage_artifact_gate")?;

	assert!(support::array_contains_str(staged_gate, "/dropped_evidence", "trajectory-win-decoy")?);

	let hierarchy_stages = support::array_at(hierarchy, "/trace_explainability/stages")?;
	let hierarchy_gate = support::find_by_field(
		hierarchy_stages,
		"/stage_name",
		"openviking.hierarchy_artifact_gate",
	)?;

	assert!(support::array_contains_str(
		hierarchy_gate,
		"/dropped_evidence",
		"hierarchy-design-win-decoy"
	)?);

	let recursive_stages = support::array_at(recursive, "/trace_explainability/stages")?;
	let recursive_gate = support::find_by_field(
		recursive_stages,
		"/stage_name",
		"openviking.recursive_expansion_gate",
	)?;

	assert!(support::array_contains_str(
		recursive_gate,
		"/dropped_evidence",
		"recursive-expansion-win-decoy"
	)?);
	assert!(
		staged.pointer("/reason").and_then(Value::as_str).is_some_and(
			|reason| reason.contains("same-corpus output returns expected evidence ids")
		)
	);

	Ok(())
}
