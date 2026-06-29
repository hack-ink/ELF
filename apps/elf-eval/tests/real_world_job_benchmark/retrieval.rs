use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;
use serde_json::Value;

use super::support::*;

#[test]
fn retrieval_fixtures_report_quality_and_trace_attribution() -> Result<()> {
	let report = run_json_report_from(retrieval_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/irrelevant_context_ratio").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/wrong_result_stage_attribution_count").and_then(Value::as_u64),
		Some(0)
	);

	let suites = array_at(&report, "/suites")?;
	let retrieval_suite = find_by_field(suites, "/suite_id", "retrieval")?;
	let debug_suite = find_by_field(suites, "/suite_id", "operator_debugging_ux")?;

	assert_eq!(retrieval_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(retrieval_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(debug_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	let jobs = array_at(&report, "/jobs")?;
	let stage_job = find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;

	assert_eq!(stage_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		stage_job.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("rerank.score")
	);
	assert_eq!(
		stage_job.pointer("/retrieval_quality/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		stage_job.pointer("/retrieval_quality/irrelevant_context_ratio").and_then(Value::as_f64),
		Some(0.0)
	);

	Ok(())
}

#[test]
fn stage_attribution_fixture_still_fails_when_decoy_is_used() -> Result<()> {
	let fixture_path = retrieval_fixture_dir().join("stage_explainability_wrong_result.json");
	let mut fixture = serde_json::from_str::<Value>(&fs::read_to_string(fixture_path)?)?;

	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/content",
		Value::String(
			"The trace shows the expected evidence was present in recall.candidates but demoted at rerank.score; however, the selected answer followed the stale top-k smoke-only evidence.".to_string(),
		),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/claims",
		serde_json::json!([]),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!(["stage-decoy"]),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-stage-decoy-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stage_decoy.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;

	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(
		report.pointer("/summary/wrong_result_stage_attribution_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("rerank.score")
	);
	assert_eq!(
		job.pointer("/retrieval_quality/trap_context_count").and_then(Value::as_u64),
		Some(1)
	);

	Ok(())
}

#[test]
fn retrieval_report_markdown_includes_quality_metrics() -> Result<()> {
	let report = run_json_report_from(retrieval_fixture_dir())?;
	let temp_dir = env::temp_dir().join(format!("elf-real-world-retrieval-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("retrieval-report.json");
	let markdown_path = temp_dir.join("retrieval-report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("Expected evidence recall"));
	assert!(markdown.contains("Irrelevant context ratio"));
	assert!(markdown.contains("Trace Explainability"));
	assert!(markdown.contains("rerank.score"));

	Ok(())
}
