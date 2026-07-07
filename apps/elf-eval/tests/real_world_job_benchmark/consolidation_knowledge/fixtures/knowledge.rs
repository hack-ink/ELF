use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn knowledge_fixtures_report_page_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::knowledge_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		report.pointer("/summary/knowledge/section_count").and_then(Value::as_u64),
		Some(13)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/citation_coverage").and_then(Value::as_f64),
		Some(0.923)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/rebuild_determinism").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_count").and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/pages_with_backlinks").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.979)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/pages_with_version_diff").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/allowed_variance_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(&report, "/suites")?;
	let knowledge_suite = support::find_by_field(suites, "/suite_id", "knowledge_compilation")?;

	assert_eq!(knowledge_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(knowledge_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = support::array_at(&report, "/jobs")?;
	let project_page_job = support::find_by_field(jobs, "/job_id", "knowledge-project-page-001")?;
	let watch_rebuild_job = support::find_by_field(jobs, "/job_id", "knowledge-watch-rebuild-003")?;

	assert_eq!(
		project_page_job.pointer("/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		project_page_job.pointer("/knowledge/untraced_section_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		watch_rebuild_job.pointer("/knowledge/pages_with_version_diff").and_then(Value::as_u64),
		Some(1)
	);
	assert!(
		watch_rebuild_job
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| answer
				.contains("PageIndex/OpenKB adapter claim as lint evidence")
				&& answer.contains("leaves source documents plus Memory Notes unmodified"))
	);

	Ok(())
}
