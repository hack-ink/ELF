#![allow(unused_crate_dependencies)]

//! Integration tests for the real-world job smoke benchmark runner.

use std::{
	env, fs,
	path::{Path, PathBuf},
	process::{self, Command},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

fn fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_job").join("smoke")
}

fn fixture_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_job")
}

fn real_world_memory_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_memory")
}

fn evolution_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("evolution")
}

fn operator_debug_fixture_dir() -> PathBuf {
	fixture_root().join("operator_debugging_ux")
}

fn run_json_report_from(fixtures: PathBuf) -> Result<Value> {
	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixtures)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	Ok(serde_json::from_slice(&output.stdout)?)
}

fn run_json_report() -> Result<Value> {
	run_json_report_from(fixture_dir())
}

fn array_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
	value
		.pointer(pointer)
		.and_then(Value::as_array)
		.ok_or_else(|| eyre::eyre!("missing array at {pointer}"))
}

fn find_by_field<'a>(items: &'a [Value], field: &str, expected: &str) -> Result<&'a Value> {
	items
		.iter()
		.find(|item| item.pointer(field).and_then(Value::as_str) == Some(expected))
		.ok_or_else(|| eyre::eyre!("missing item with {field} = {expected}"))
}

fn set_json_pointer(value: &mut Value, pointer: &str, replacement: Value) -> Result<()> {
	let target =
		value.pointer_mut(pointer).ok_or_else(|| eyre::eyre!("missing JSON pointer {pointer}"))?;

	*target = replacement;

	Ok(())
}

#[test]
fn smoke_fixture_produces_typed_json_report() -> Result<()> {
	let report = run_json_report()?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.real_world_job_report/v1")
	);
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "work-resume-smoke-001")?;

	assert_eq!(job.pointer("/suite_id").and_then(Value::as_str), Some("work_resume"));
	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(job.pointer("/latency_ms").and_then(Value::as_f64), Some(1.2));
	assert_eq!(job.pointer("/cost/amount").and_then(Value::as_f64), Some(0.0));

	let expected_evidence = array_at(job, "/expected_evidence")?;
	let produced_evidence = array_at(job, "/produced_evidence")?;

	assert_eq!(expected_evidence.len(), 2);
	assert_eq!(produced_evidence.len(), 1);
	assert_eq!(produced_evidence.first().and_then(Value::as_str), Some("issue-xy812-resume"));

	let suites = array_at(&report, "/suites")?;
	let encoded_suite = find_by_field(suites, "/suite_id", "work_resume")?;
	let unencoded_suite = find_by_field(suites, "/suite_id", "retrieval")?;

	assert_eq!(encoded_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(unencoded_suite.pointer("/status").and_then(Value::as_str), Some("not_encoded"));

	Ok(())
}

#[test]
fn runner_discovers_nested_fixture_layout() -> Result<()> {
	let report = run_json_report_from(fixture_root())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));

	let suites = array_at(&report, "/suites")?;
	let operator_suite = find_by_field(suites, "/suite_id", "operator_debugging_ux")?;

	assert_eq!(operator_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	Ok(())
}

#[test]
fn operator_debug_fixture_reports_trace_links_and_failure_details() -> Result<()> {
	let report = run_json_report_from(operator_debug_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		report.pointer("/summary/operator_debug_job_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(report.pointer("/summary/raw_sql_needed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/trace_incomplete_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/operator_ux_gap_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(1));

	let jobs = array_at(&report, "/jobs")?;
	let dropped = find_by_field(jobs, "/job_id", "operator-debug-dropped-evidence-001")?;

	assert_eq!(dropped.pointer("/status").and_then(Value::as_str), Some("unsupported_claim"));
	assert_eq!(
		dropped.pointer("/operator_debug/raw_sql_needed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		dropped.pointer("/operator_debug/dropped_candidate_visibility").and_then(Value::as_str),
		Some("visible in Retrieval Funnel and Replay Candidates")
	);
	assert_eq!(
		dropped.pointer("/operator_debug/viewer_url").and_then(Value::as_str),
		Some("/viewer?trace_id=11111111-1111-4111-8111-111111111111")
	);

	Ok(())
}

#[test]
fn generated_json_report_renders_markdown() -> Result<()> {
	let report = run_json_report()?;
	let temp_dir = env::temp_dir().join(format!("elf-real-world-job-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("report.json");
	let markdown_path = temp_dir.join("report.md");

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

	assert!(markdown.contains("# Real-World Job Benchmark Report"));
	assert!(markdown.contains("work_resume"));
	assert!(markdown.contains("issue-xy812-resume"));
	assert!(markdown.contains("## Operator Debugging UX"));
	assert!(markdown.contains("Existing live-baseline reports remain valid"));

	Ok(())
}

#[test]
fn real_world_memory_fixtures_report_trust_and_personalization_metrics() -> Result<()> {
	let report = run_json_report_from(real_world_memory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(9));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_retrieval_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_violation_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_case_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_pass_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/evidence_required_count").and_then(Value::as_u64),
		Some(19)
	);
	assert_eq!(report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64), Some(17));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(0.895));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(0.895));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(0.895));

	let suites = array_at(&report, "/suites")?;

	for suite_id in ["trust_source_of_truth", "capture_integration", "personalization"] {
		let suite = find_by_field(suites, "/suite_id", suite_id)?;

		assert_eq!(suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	}

	let memory_evolution = find_by_field(suites, "/suite_id", "memory_evolution")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("not_encoded"));

	let jobs = array_at(&report, "/jobs")?;
	let rebuild = find_by_field(jobs, "/job_id", "trust-sot-rebuild-001")?;
	let redaction = find_by_field(jobs, "/job_id", "capture-redaction-exclusion-001")?;
	let personalization = find_by_field(jobs, "/job_id", "personalization-scoped-preference-001")?;

	assert_eq!(rebuild.pointer("/qdrant_rebuild_case").and_then(Value::as_bool), Some(true));
	assert_eq!(redaction.pointer("/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(personalization.pointer("/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(personalization.pointer("/scope_correct_count").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn memory_evolution_fixtures_report_temporal_and_staleness_metrics() -> Result<()> {
	let report = run_json_report_from(evolution_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/evolution/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(&report, "/suites")?;
	let memory_evolution = find_by_field(suites, "/suite_id", "memory_evolution")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(
		memory_evolution.pointer("/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = array_at(&report, "/jobs")?;
	let relation_job = find_by_field(jobs, "/job_id", "memory-evolution-relation-temporal-001")?;

	assert_eq!(relation_job.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(
		relation_job.pointer("/evolution/temporal_validity_not_encoded").and_then(Value::as_bool),
		Some(true)
	);

	let follow_ups = array_at(&report, "/follow_ups")?;

	assert_eq!(follow_ups.len(), 1);
	assert_eq!(
		follow_ups
			.first()
			.and_then(|follow_up| follow_up.pointer("/title"))
			.and_then(Value::as_str),
		Some("[ELF graph P1] Add temporal validity to graph-lite facts")
	);

	Ok(())
}

#[test]
fn memory_evolution_counts_stale_answer_when_old_fact_is_answered_as_current() -> Result<()> {
	let fixture_path =
		evolution_fixture_dir().join("preference_changed_current_vs_historical.json");
	let mut fixture = serde_json::from_str::<Value>(&fs::read_to_string(fixture_path)?)?;

	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/content",
		Value::String(
			"Use terse bullet-only benchmark updates as the current preference.".to_string(),
		),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!(["pref-old-terse-bullets"]),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/claims",
		serde_json::json!([
			{
				"claim_id": "current_preference",
				"text": "Use terse bullet-only benchmark updates as the current preference.",
				"evidence_ids": ["pref-old-terse-bullets"],
				"confidence": "high"
			}
		]),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-stale-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stale_preference.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;

	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(job.pointer("/evolution/stale_answer_count").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn operator_debug_json_report_renders_markdown_links() -> Result<()> {
	let report = run_json_report_from(operator_debug_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-job-operator-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("operator.json");
	let markdown_path = temp_dir.join("operator.md");

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

	assert!(markdown.contains("operator-debug-dropped-evidence-001"));
	assert!(markdown.contains("/viewer?trace_id=11111111-1111-4111-8111-111111111111"));
	assert!(markdown.contains("Raw SQL"));
	assert!(markdown.contains("Replay Candidates"));
	assert!(markdown.contains("Root cause"));

	Ok(())
}

#[test]
fn memory_evolution_report_renders_markdown_counters() -> Result<()> {
	let report = run_json_report_from(evolution_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-evolution-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("evolution-report.json");
	let markdown_path = temp_dir.join("evolution-report.md");

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

	assert!(markdown.contains("## Memory Evolution"));
	assert!(markdown.contains("Temporal validity not encoded: `1`"));
	assert!(markdown.contains("[ELF graph P1] Add temporal validity to graph-lite facts"));

	Ok(())
}
