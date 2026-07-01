use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn scheduled_memory_fixtures_score_task_trace_gate() -> Result<()> {
	let report = support::run_json_report_from(support::scheduled_memory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/scheduled_memory/job_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/task_run_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/output_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/invalid_current_output_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	let suites = support::array_at(&report, "/suites")?;
	let scheduled = support::find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let jobs = support::array_at(&report, "/jobs")?;
	let weekly =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;
	let private = support::find_by_field(
		jobs,
		"/job_id",
		"scheduled-private-provider-scheduler-blocked-001",
	)?;

	assert_eq!(weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		weekly.pointer("/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(private.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		report
			.pointer("/follow_ups/0/title")
			.and_then(Value::as_str)
			.is_some_and(|title| title.contains("XY-930"))
	);

	Ok(())
}
