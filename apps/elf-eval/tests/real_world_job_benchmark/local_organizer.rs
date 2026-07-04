use std::{
	env, fs,
	process::{self, Command, Output},
};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn r1_local_organizer_report_emits_gate_metrics() -> Result<()> {
	let report = support::run_json_report_from(
		support::real_world_memory_fixture_dir().join("local_background_organizer"),
	)?;
	let summary =
		report.pointer("/summary/local_organizer").expect("missing local_organizer summary");

	assert_eq!(summary.pointer("/job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(
		summary.pointer("/extraction_f1_not_encoded_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(summary.pointer("/json_schema_valid_count").and_then(Value::as_u64), Some(1));
	assert_eq!(summary.pointer("/citation_source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(summary.pointer("/unsupported_claim_rate").and_then(Value::as_f64), Some(0.0));
	assert_eq!(
		summary.pointer("/stale_correction_delete_score").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(summary.pointer("/source_mutation_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		summary.pointer("/silent_memory_authority_mutation_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(summary.pointer("/escalation_rate").and_then(Value::as_f64), Some(1.0));
	assert_eq!(summary.pointer("/mean_latency_ms").and_then(Value::as_f64), Some(42.0));
	assert_eq!(summary.pointer("/p95_latency_ms").and_then(Value::as_f64), Some(42.0));

	let tiers = support::array_at(summary, "/tiers")?;

	assert_eq!(tiers.len(), 1);
	assert_eq!(tiers[0].pointer("/model_tier").and_then(Value::as_str), Some("L2"));
	assert_eq!(tiers[0].pointer("/total_cost/input_tokens").and_then(Value::as_u64), Some(320));
	assert!(!support::array_at(&tiers[0], "/resource_footprints")?.is_empty());

	let job = &support::array_at(&report, "/jobs")?[0];

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		job.pointer("/local_organizer/extraction_f1_blocker").and_then(Value::as_str),
		Some(
			"not_encoded: no labeled extraction set is checked in for this first R1 local organizer slice."
		)
	);
	assert_eq!(
		job.pointer("/local_organizer/runtime_commit").and_then(Value::as_str),
		Some("fixture-runtime@2026-07-04")
	);
	assert_eq!(
		job.pointer("/consolidation/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	Ok(())
}

#[test]
fn r1_local_organizer_validate_command_accepts_fixture_report() -> Result<()> {
	let report = support::run_json_report_from(
		support::real_world_memory_fixture_dir().join("local_background_organizer"),
	)?;
	let output = validate_local_organizer_report(&report, "valid")?;

	assert!(
		output.status.success(),
		"validate-local-organizer failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);

	Ok(())
}

#[test]
fn r1_local_organizer_validate_command_rejects_missing_l2_escalation() -> Result<()> {
	let mut report = support::run_json_report_from(
		support::real_world_memory_fixture_dir().join("local_background_organizer"),
	)?;

	*report
		.pointer_mut("/jobs/0/local_organizer/required_escalation_count")
		.expect("missing required_escalation_count") = Value::from(0);
	*report
		.pointer_mut("/jobs/0/local_organizer/completed_escalation_count")
		.expect("missing completed_escalation_count") = Value::from(0);

	let output = validate_local_organizer_report(&report, "missing-escalation")?;

	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("must encode at least one required"));

	Ok(())
}

#[test]
fn r1_local_organizer_validate_command_rejects_missing_tier_cost() -> Result<()> {
	let mut report = support::run_json_report_from(
		support::real_world_memory_fixture_dir().join("local_background_organizer"),
	)?;

	report
		.pointer_mut("/summary/local_organizer/tiers/0")
		.expect("missing local organizer tier")
		.as_object_mut()
		.expect("tier must be object")
		.remove("total_cost");

	let output = validate_local_organizer_report(&report, "missing-tier-cost")?;

	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("must report cost footprint"));

	Ok(())
}

fn validate_local_organizer_report(report: &Value, suffix: &str) -> Result<Output> {
	let temp_dir =
		env::temp_dir().join(format!("elf-r1-local-organizer-report-{}-{suffix}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("report.json");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("validate-local-organizer")
		.arg("--report")
		.arg(&report_path)
		.output()?;

	Ok(output)
}

#[test]
fn r1_local_organizer_task_is_registered() -> Result<()> {
	let makefile =
		fs::read_to_string(support::workspace_root()?.join("makefiles/benchmark-memory-b.toml"))?;

	for task in [
		"[tasks.real-world-memory-r1-local-organizer]",
		"[tasks.real-world-memory-r1-local-organizer-json]",
		"[tasks.real-world-memory-r1-local-organizer-validate]",
		"[tasks.real-world-memory-r1-local-organizer-report]",
	] {
		assert!(makefile.contains(task), "missing cargo make task {task}");
	}

	Ok(())
}
