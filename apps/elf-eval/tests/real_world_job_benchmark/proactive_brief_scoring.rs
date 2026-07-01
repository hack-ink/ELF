use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn proactive_brief_fixtures_score_source_linked_suggestions() -> Result<()> {
	let report = support::run_json_report_from(support::proactive_brief_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/proactive_brief/brief_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/suggestion_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/invalid_current_suggestion_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/rejected_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/deferred_count").and_then(Value::as_u64),
		Some(2)
	);

	let suites = support::array_at(&report, "/suites")?;
	let proactive = support::find_by_field(suites, "/suite_id", "proactive_brief")?;

	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(proactive.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let jobs = support::array_at(&report, "/jobs")?;
	let daily = support::find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private =
		support::find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;

	assert_eq!(daily.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		daily.pointer("/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
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
