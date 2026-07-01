use std::{env, fs, process};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn memory_summary_fixture_fails_stale_top_of_mind_entries() -> Result<()> {
	let fixture_path =
		support::memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][2]["category"] =
		Value::String("top_of_mind".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][2]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir =
		env::temp_dir().join(format!("elf-memory-summary-stale-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stale_current_summary.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}
#[test]
fn memory_summary_fixture_fails_tombstoned_top_of_mind_entries() -> Result<()> {
	let fixture_path =
		support::memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][4]["category"] =
		Value::String("top_of_mind".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][4]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir = env::temp_dir()
		.join(format!("elf-memory-summary-tombstone-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("tombstone_current_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}
#[test]
fn memory_summary_fixture_fails_untraced_derived_profile_entries() -> Result<()> {
	let fixture_path =
		support::memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["unsupported_claim_flags"] =
		Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-memory-summary-untraced-derived-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("untraced_derived_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("unsupported_claim"));
	assert_eq!(
		job.pointer("/memory_summary/derived_missing_source_or_unsupported_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(1));

	Ok(())
}
#[test]
fn memory_summary_fixture_fails_unsupported_current_derived_entries() -> Result<()> {
	let fixture_path =
		support::memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["source_refs"] =
		Value::Array(vec![Value::String("summary-contract-non-parity-boundary".to_string())]);
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["freshness"]
		["status"] = Value::String("current".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["rationale"]
		["decision"] = Value::String("included".to_string());

	let temp_dir = env::temp_dir()
		.join(format!("elf-memory-summary-unsupported-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("unsupported_current_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/unsupported_current_entry_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}
#[test]
fn memory_summary_fixture_fails_tombstone_entries_without_tombstone_refs() -> Result<()> {
	let fixture_path =
		support::memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][4]["freshness"]
		["tombstone_refs"] = Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-memory-summary-tombstone-refs-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("missing_tombstone_refs_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/freshness_coverage").and_then(Value::as_f64),
		Some(0.857)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}
