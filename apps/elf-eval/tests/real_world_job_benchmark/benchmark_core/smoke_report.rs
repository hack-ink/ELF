use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn smoke_fixture_produces_typed_json_report() -> Result<()> {
	let report = support::run_json_report()?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.real_world_job_report/v1")
	);
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/external_adapters/summary/adapter_count").and_then(Value::as_u64),
		Some(26)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/live_real_world_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(14)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "work-resume-stale-worktree-001")?;

	assert_eq!(job.pointer("/suite_id").and_then(Value::as_str), Some("work_resume"));
	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(job.pointer("/latency_ms").and_then(Value::as_f64), Some(2.0));
	assert_eq!(job.pointer("/cost/amount").and_then(Value::as_f64), Some(0.0));

	let expected_evidence = support::array_at(job, "/expected_evidence")?;
	let produced_evidence = support::array_at(job, "/produced_evidence")?;

	assert_eq!(expected_evidence.len(), 2);
	assert_eq!(produced_evidence.len(), 1);
	assert_eq!(produced_evidence.first().and_then(Value::as_str), Some("xy844-current-worktree"));

	let suites = support::array_at(&report, "/suites")?;
	let encoded_suite = support::find_by_field(suites, "/suite_id", "work_resume")?;
	let capture_suite = support::find_by_field(suites, "/suite_id", "capture_integration")?;
	let unencoded_suite = support::find_by_field(suites, "/suite_id", "retrieval")?;

	assert_eq!(encoded_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(encoded_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(capture_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(capture_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(unencoded_suite.pointer("/status").and_then(Value::as_str), Some("not_encoded"));

	let capture_fixture_backed = support::array_at(&report, "/capture_integration/fixture_backed")?;

	assert!(capture_fixture_backed.iter().any(|value| {
		value.as_str().is_some_and(|item| item.contains("agentmemory-style hook capture"))
	}));

	let capture_not_encoded = support::array_at(&report, "/capture_integration/not_encoded")?;

	assert!(capture_not_encoded.iter().any(|value| {
		value.as_str().is_some_and(|item| item.contains("No live external hook ingestion"))
	}));

	Ok(())
}
