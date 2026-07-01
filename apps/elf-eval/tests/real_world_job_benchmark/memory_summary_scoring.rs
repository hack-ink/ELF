use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn memory_summary_fixtures_score_reviewable_source_trace_contract() -> Result<()> {
	let report = support::run_json_report_from(support::memory_summary_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/memory_summary/summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/entry_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/summary/memory_summary/covered_required_category_count")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/rationale_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/memory_summary/unsupported_derived_entry_count")
			.and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(&report, "/suites")?;
	let memory_summary = support::find_by_field(suites, "/suite_id", "memory_summary")?;

	assert_eq!(memory_summary.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(memory_summary.pointer("/encoded_job_count").and_then(Value::as_u64), Some(1));

	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(job.pointer("/memory_summary/top_of_mind_count").and_then(Value::as_u64), Some(1));
	assert_eq!(job.pointer("/memory_summary/tombstone_ref_count").and_then(Value::as_u64), Some(1));

	Ok(())
}
