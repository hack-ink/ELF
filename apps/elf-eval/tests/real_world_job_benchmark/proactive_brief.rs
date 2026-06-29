use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn proactive_brief_fixtures_score_source_linked_suggestions() -> Result<()> {
	let report = support::run_json_report_from(support::proactive_brief_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/proactive_brief/brief_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/suggestion_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/invalid_current_suggestion_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/rejected_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/deferred_count").and_then(Value::as_u64),
		Some(2)
	);

	let suites = support::array_at(&report, "/suites")?;
	let proactive = support::find_by_field(suites, "/suite_id", "proactive_brief")?;

	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(proactive.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let jobs = support::array_at(&report, "/jobs")?;
	let daily = support::find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private =
		support::find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;

	assert_eq!(daily.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		daily.pointer("/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
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
fn proactive_brief_markdown_renders_source_and_freshness_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::proactive_brief_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-proactive-brief-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("proactive-brief-report.json");
	let markdown_path = temp_dir.join("proactive-brief-report.md");

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

	assert!(markdown.contains("Proactive Brief Metrics"));
	assert!(markdown.contains("proactive-daily-project-brief-001"));
	assert!(markdown.contains("Proactive evidence-ref coverage"));
	assert!(markdown.contains("Invalid Current"));
	assert!(markdown.contains("Tombstone Violations"));

	Ok(())
}

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
