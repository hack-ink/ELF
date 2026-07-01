use std::{env, fs, process};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn proactive_brief_fixture_fails_unsupported_suggestions() -> Result<()> {
	let fixture_path = support::proactive_brief_fixture_dir().join("daily_project_brief.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["evidence_refs"] =
		Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-proactive-unsupported-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("unsupported_brief.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("unsupported_claim"));
	assert_eq!(
		job.pointer("/proactive_brief/untraced_suggestion_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(1));

	Ok(())
}
#[test]
fn proactive_brief_fixture_fails_stale_decisions_presented_current() -> Result<()> {
	let fixture_path = support::proactive_brief_fixture_dir().join("stale_decision_audit.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir =
		env::temp_dir().join(format!("elf-proactive-stale-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stale_current_brief.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job = support::find_by_field(jobs, "/job_id", "proactive-stale-decision-audit-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/proactive_brief/invalid_current_suggestion_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}
#[test]
fn proactive_brief_fixture_fails_tombstone_ttl_violations() -> Result<()> {
	let fixture_path =
		support::proactive_brief_fixture_dir().join("stale_plan_preference_warning.json");
	let mut fixture = support::load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["freshness"]
		["status"] = Value::String("current".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["action"]
		["decision"] = Value::String("recommend".to_string());

	let temp_dir = env::temp_dir().join(format!("elf-proactive-tombstone-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("tombstone_current_brief.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job =
		support::find_by_field(jobs, "/job_id", "proactive-stale-plan-preference-warning-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/proactive_brief/tombstone_violation_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}
