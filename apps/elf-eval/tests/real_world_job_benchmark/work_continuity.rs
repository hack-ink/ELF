use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;
use serde_json::Value;

use super::support::*;

#[test]
fn work_continuity_fixtures_score_required_metrics() -> Result<()> {
	let report = run_json_report_from(work_continuity_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));

	assert_work_continuity_summary_counts(&report);

	let suites = array_at(&report, "/suites")?;
	let work_continuity = find_by_field(suites, "/suite_id", "work_continuity")?;

	assert_eq!(work_continuity.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(work_continuity.pointer("/encoded_job_count").and_then(Value::as_u64), Some(8));

	Ok(())
}

fn assert_work_continuity_summary_counts(report: &Value) {
	for (field, expected) in [
		("readback_count", 8),
		("entry_count", 8),
		("reset_resume_required_count", 1),
		("reset_resume_success_count", 1),
		("decision_rationale_required_count", 1),
		("decision_rationale_recalled_count", 1),
		("rejected_option_required_count", 1),
		("rejected_option_suppressed_count", 1),
		("rejected_option_resurrection_count", 0),
		("explicit_next_step_required_count", 1),
		("explicit_next_step_returned_count", 1),
		("explicit_next_step_correct_count", 1),
		("inferred_next_step_required_count", 1),
		("inferred_next_step_labeled_count", 1),
		("inferred_step_instruction_count", 0),
		("handoff_source_ref_required_count", 1),
		("handoff_source_ref_covered_count", 1),
		("redaction_required_count", 1),
		("redaction_applied_count", 1),
		("sensitive_marker_persistence_count", 0),
		("janitor_candidate_count", 1),
		("janitor_false_promotion_count", 0),
		("journal_only_authority_claim_count", 0),
	] {
		assert_work_continuity_summary_u64(report, field, expected);
	}
	for (field, expected) in [
		("reset_resume_success_rate", 1.0),
		("decision_rationale_recall_rate", 1.0),
		("rejected_option_suppression_rate", 1.0),
		("explicit_next_step_precision", 1.0),
		("inferred_next_step_labeling_rate", 1.0),
		("handoff_source_ref_coverage", 1.0),
		("redaction_rate", 1.0),
		("janitor_false_promotion_rate", 0.0),
	] {
		assert_work_continuity_summary_f64(report, field, expected);
	}
}

fn assert_work_continuity_summary_u64(report: &Value, field: &str, expected: u64) {
	assert_eq!(
		report.pointer(&format!("/summary/work_continuity/{field}")).and_then(Value::as_u64),
		Some(expected),
		"unexpected Work Continuity summary field {field}",
	);
}

fn assert_work_continuity_summary_f64(report: &Value, field: &str, expected: f64) {
	assert_eq!(
		report.pointer(&format!("/summary/work_continuity/{field}")).and_then(Value::as_f64),
		Some(expected),
		"unexpected Work Continuity summary field {field}",
	);
}

#[test]
fn work_continuity_markdown_renders_required_metrics() -> Result<()> {
	let report = run_json_report_from(work_continuity_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-work-continuity-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("work-continuity-report.json");
	let markdown_path = temp_dir.join("work-continuity-report.md");

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

	assert!(markdown.contains("Work Continuity Metrics"));
	assert!(markdown.contains("work-continuity-redaction-001"));
	assert!(markdown.contains("work-continuity-janitor-false-promotion-001"));
	assert!(markdown.contains("Janitor False Promotion"));
	assert!(markdown.contains("Sensitive Persistence"));
	assert!(markdown.contains("Journal Authority Claims"));
	assert!(markdown.contains("| work-continuity-reset-resume-001 | 1 | 1 | `1/1` (`1.000`)"));
	assert!(markdown.contains(
		"| work-continuity-explicit-next-step-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `1/1` (`1.000`)"
	));
	assert!(markdown.contains(
		"| work-continuity-handoff-source-ref-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`1.000`) | `0/0` (`0.000`) | `1/1` (`1.000`)"
	));
	assert!(markdown.contains(
		"| work-continuity-redaction-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`1.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `1/1` (`1.000`)"
	));
	assert!(markdown.contains(
		"| work-continuity-janitor-false-promotion-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`1.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/1` (`0.000`)"
	));

	Ok(())
}

#[test]
fn work_continuity_fixture_fails_sensitive_marker_persistence() -> Result<()> {
	let report = run_work_continuity_mutation(
		"redaction_sensitive_marker.json",
		"sensitive_marker_persistence.json",
		|fixture| {
			fixture["corpus"]["adapter_response"]["answer"]["work_journal_readbacks"][0]["items"]
				[0]["redaction_audit"]["persisted_sensitive_marker_ids"] =
				serde_json::json!(["secret-demo-token"]);
		},
	)?;
	let job = single_work_continuity_job(&report, "work-continuity-redaction-001")?;

	assert_work_continuity_wrong_result(job, "sensitive_marker_persistence_count", 1);

	Ok(())
}

#[test]
fn work_continuity_fixture_fails_rejected_option_resurrection() -> Result<()> {
	let report = run_work_continuity_mutation(
		"rejected_option_suppression.json",
		"rejected_option_resurrection.json",
		|fixture| {
			fixture["corpus"]["adapter_response"]["answer"]["work_journal_readbacks"][0]["items"]
				[0]["rejected_options"][0]["resurrected_as_current"] = Value::Bool(true);
		},
	)?;
	let job = single_work_continuity_job(&report, "work-continuity-rejected-option-001")?;

	assert_work_continuity_wrong_result(job, "rejected_option_resurrection_count", 1);

	Ok(())
}

#[test]
fn work_continuity_fixture_fails_inferred_step_instruction() -> Result<()> {
	let report = run_work_continuity_mutation(
		"inferred_next_step_labeling.json",
		"inferred_step_instruction.json",
		|fixture| {
			fixture["corpus"]["adapter_response"]["answer"]["work_journal_readbacks"][0]["items"]
				[0]["inferred_next_steps"][0]["instruction"] = Value::Bool(true);
		},
	)?;
	let job = single_work_continuity_job(&report, "work-continuity-inferred-next-step-001")?;

	assert_work_continuity_wrong_result(job, "inferred_step_instruction_count", 1);

	Ok(())
}

#[test]
fn work_continuity_fixture_fails_journal_only_authority_claim() -> Result<()> {
	let report = run_work_continuity_mutation(
		"handoff_source_ref_coverage.json",
		"journal_only_authority_claim.json",
		|fixture| {
			fixture["corpus"]["adapter_response"]["answer"]["work_journal_readbacks"][0]["where_stopped"]
				["journal_only_authority_claims"] = serde_json::json!(["wj-handoff-source-ref"]);
		},
	)?;
	let job = single_work_continuity_job(&report, "work-continuity-handoff-source-ref-001")?;

	assert_work_continuity_wrong_result(job, "journal_only_authority_claim_count", 1);

	Ok(())
}

#[test]
fn work_continuity_fixture_fails_janitor_promotion_or_missing_review() -> Result<()> {
	let promoted = run_work_continuity_mutation(
		"janitor_false_promotion_guard.json",
		"janitor_promoted.json",
		|fixture| {
			fixture["corpus"]["adapter_response"]["answer"]["work_journal_readbacks"][0]["janitor_candidates"]
				[0]["promoted_to_memory"] = Value::Bool(true);
		},
	)?;
	let promoted_job =
		single_work_continuity_job(&promoted, "work-continuity-janitor-false-promotion-001")?;

	assert_work_continuity_wrong_result(promoted_job, "janitor_false_promotion_count", 1);
	assert_hard_fail_hit(promoted_job, "janitor Work Journal candidate promoted without review");

	let missing_review = run_work_continuity_mutation(
		"janitor_false_promotion_guard.json",
		"janitor_missing_review_required.json",
		|fixture| {
			fixture["corpus"]["adapter_response"]["answer"]["work_journal_readbacks"][0]["janitor_candidates"]
				[0]["review_required"] = Value::Bool(false);
		},
	)?;
	let missing_review_job =
		single_work_continuity_job(&missing_review, "work-continuity-janitor-false-promotion-001")?;

	assert_work_continuity_wrong_result(missing_review_job, "janitor_false_promotion_count", 1);
	assert_hard_fail_hit(
		missing_review_job,
		"janitor Work Journal candidate promoted without review",
	);

	let extra_bad_candidate = run_work_continuity_mutation(
		"janitor_false_promotion_guard.json",
		"janitor_extra_bad_candidate.json",
		|fixture| {
			fixture["corpus"]["adapter_response"]["answer"]["work_journal_readbacks"][0]["janitor_candidates"] = serde_json::json!([
				{
					"candidate_id": "wj-janitor-candidate",
					"evidence_refs": ["wj-janitor-candidate-source"],
					"review_required": true,
					"promoted_to_memory": false
				},
				{
					"candidate_id": "wj-extra-janitor-candidate",
					"evidence_refs": ["wj-janitor-candidate-source"],
					"review_required": true,
					"promoted_to_memory": true
				}
			]);
		},
	)?;
	let extra_bad_candidate_job = single_work_continuity_job(
		&extra_bad_candidate,
		"work-continuity-janitor-false-promotion-001",
	)?;

	assert_work_continuity_wrong_result(
		extra_bad_candidate_job,
		"janitor_false_promotion_count",
		1,
	);
	assert_hard_fail_hit(
		extra_bad_candidate_job,
		"janitor Work Journal candidate promoted without review",
	);

	assert_eq!(
		extra_bad_candidate_job
			.pointer("/work_continuity/janitor_candidate_count")
			.and_then(Value::as_u64),
		Some(2)
	);

	Ok(())
}

fn run_work_continuity_mutation(
	fixture_name: &str,
	output_name: &str,
	mutate: impl FnOnce(&mut Value),
) -> Result<Value> {
	let fixture_path = work_continuity_fixture_dir().join(fixture_name);
	let temp_dir =
		env::temp_dir().join(format!("elf-work-continuity-{output_name}-{}", process::id()));
	let mut fixture = load_json(&fixture_path)?;

	mutate(&mut fixture);

	if temp_dir.exists() {
		fs::remove_dir_all(&temp_dir)?;
	}

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join(output_name), serde_json::to_vec_pretty(&fixture)?)?;

	run_json_report_from(temp_dir)
}

fn single_work_continuity_job<'a>(report: &'a Value, job_id: &str) -> Result<&'a Value> {
	let jobs = array_at(report, "/jobs")?;

	find_by_field(jobs, "/job_id", job_id)
}

fn assert_work_continuity_wrong_result(job: &Value, metric_name: &str, expected: u64) {
	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer(&format!("/work_continuity/{metric_name}")).and_then(Value::as_u64),
		Some(expected)
	);
}

fn assert_hard_fail_hit(job: &Value, expected_hit: &str) {
	let hits = job.pointer("/hard_fail_hits").and_then(Value::as_array).expect("hard_fail_hits");

	assert!(
		hits.iter().filter_map(Value::as_str).any(|hit| hit == expected_hit),
		"missing hard_fail_hits marker: {expected_hit}"
	);
}
