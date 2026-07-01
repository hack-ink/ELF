use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn capture_integration_fixtures_score_redaction_and_source_ids() -> Result<()> {
	let report = support::run_json_report_from(support::capture_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));

	let suites = support::array_at(&report, "/suites")?;
	let capture = support::find_by_field(suites, "/suite_id", "capture_integration")?;

	assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(capture.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = support::array_at(&report, "/jobs")?;
	let source_id = support::find_by_field(jobs, "/job_id", "capture-source-id-binding-001")?;
	let redaction = support::find_by_field(jobs, "/job_id", "capture-write-policy-redaction-001")?;

	assert!(support::array_contains_str(
		source_id,
		"/produced_evidence",
		"source-id-release-summary"
	)?);
	assert!(support::array_contains_str(source_id, "/produced_evidence", "source-id-command-log")?);
	assert_eq!(redaction.pointer("/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert!(
		redaction
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| !answer.contains("orchid-envelope"))
	);

	Ok(())
}

#[test]
fn source_library_fixtures_score_saved_sources_without_memory_promotion() -> Result<()> {
	let report = support::run_json_report_from(support::source_library_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));

	let suites = support::array_at(&report, "/suites")?;
	let source_library = support::find_by_field(suites, "/suite_id", "source_library")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/encoded_job_count").and_then(Value::as_u64), Some(2));

	let jobs = support::array_at(&report, "/jobs")?;
	let long_doc = support::find_by_field(jobs, "/job_id", "source-library-long-doc-001")?;
	let thread = support::find_by_field(jobs, "/job_id", "source-library-social-thread-001")?;

	assert!(support::array_contains_str(long_doc, "/produced_evidence", "article-source-record")?);
	assert!(support::array_contains_str(
		long_doc,
		"/produced_evidence",
		"article-hydrated-excerpt"
	)?);
	assert!(support::array_contains_str(thread, "/produced_evidence", "thread-source-record")?);
	assert!(support::array_contains_str(
		thread,
		"/produced_evidence",
		"thread-promotion-boundary"
	)?);
	assert!(long_doc.pointer("/produced_answer").and_then(Value::as_str).is_some_and(|answer| {
		answer.contains("does not automatically create a durable Memory Note")
	}));
	assert!(
		thread
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| answer.contains("explicit add_note or reviewed promotion"))
	);

	Ok(())
}
