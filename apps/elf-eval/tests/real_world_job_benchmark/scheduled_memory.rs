use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::{Result, eyre};
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

#[test]
fn scheduled_memory_markdown_renders_trace_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::scheduled_memory_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-scheduled-memory-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("scheduled-memory-report.json");
	let markdown_path = temp_dir.join("scheduled-memory-report.md");

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

	assert!(markdown.contains("Scheduled Memory Metrics"));
	assert!(markdown.contains("scheduled-weekly-project-status-summary-001"));
	assert!(markdown.contains("Scheduled memory evidence-ref coverage"));
	assert!(markdown.contains("Trace Coverage"));
	assert!(markdown.contains("Source Mutations"));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_missing_execution_trace() -> Result<()> {
	let fixture_path =
		support::scheduled_memory_fixture_dir().join("weekly_project_status_summary.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]
		.as_object_mut()
		.ok_or_else(|| eyre::eyre!("missing scheduled task object"))?
		.remove("execution_trace");

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-missing-trace-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("missing_trace.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/scheduled_memory/trace_complete_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_untraced_outputs() -> Result<()> {
	let fixture_path =
		support::scheduled_memory_fixture_dir().join("weekly_project_status_summary.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["outputs"][0]["evidence_refs"] =
		Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-untraced-output-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("untraced_output.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("unsupported_claim"));
	assert_eq!(
		job.pointer("/scheduled_memory/untraced_output_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_superseded_sources_presented_current() -> Result<()> {
	let fixture_path = support::scheduled_memory_fixture_dir().join("stale_decision_audit.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["outputs"][0]["evidence_refs"] =
		serde_json::json!(["scheduled-old-consolidation-only-decision"]);
	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["outputs"][0]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-superseded-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("superseded_current.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "scheduled-stale-decision-audit-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/scheduled_memory/invalid_current_output_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_source_mutation() -> Result<()> {
	let fixture_path =
		support::scheduled_memory_fixture_dir().join("weekly_project_status_summary.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["source_mutations"] = serde_json::json!([
		{
			"table": "memory_notes",
			"op": "update",
			"note_id": "scheduled-weekly-current-gate"
		}
	]);

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-source-mutation-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("source_mutation.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("lifecycle_fail"));
	assert_eq!(
		job.pointer("/scheduled_memory/source_mutation_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/lifecycle_fail").and_then(Value::as_u64), Some(1));

	Ok(())
}
