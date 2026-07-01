use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn core_archival_memory_fixtures_score_separate_core_and_archival_jobs() -> Result<()> {
	let report = support::run_json_report_from(support::core_archival_memory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/evidence_required_count").and_then(Value::as_u64),
		Some(14)
	);
	assert_eq!(report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64), Some(14));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_violation_count").and_then(Value::as_u64), Some(0));

	let suites = support::array_at(&report, "/suites")?;
	let core = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let jobs = support::array_at(&report, "/jobs")?;

	for job_id in [
		"core-archival-core-block-attachment-001",
		"core-archival-core-block-scope-001",
		"core-archival-core-block-provenance-001",
		"core-archival-stale-core-detection-001",
		"core-archival-archival-fallback-001",
		"core-archival-project-decision-recovery-001",
	] {
		let job = support::find_by_field(jobs, "/job_id", job_id)?;

		assert_eq!(job.pointer("/suite_id").and_then(Value::as_str), Some("core_archival_memory"));
		assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	}

	let scope = support::find_by_field(jobs, "/job_id", "core-archival-core-block-scope-001")?;
	let decision =
		support::find_by_field(jobs, "/job_id", "core-archival-project-decision-recovery-001")?;

	assert_eq!(scope.pointer("/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(scope.pointer("/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(scope.pointer("/scope_violation_count").and_then(Value::as_u64), Some(0));
	assert!(
		decision
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|content| content.contains("Letta remains blocked or not_tested"))
	);
	assert!(
		support::array_at(decision, "/produced_evidence")?
			.iter()
			.any(|id| id.as_str() == Some("decision-letta-export-boundary"))
	);

	Ok(())
}
