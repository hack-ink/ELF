use std::{env, fs, process};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

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
	let fixture_path = support::work_continuity_fixture_dir().join(fixture_name);
	let temp_dir =
		env::temp_dir().join(format!("elf-work-continuity-{output_name}-{}", process::id()));
	let mut fixture = support::load_json(&fixture_path)?;

	mutate(&mut fixture);

	if temp_dir.exists() {
		fs::remove_dir_all(&temp_dir)?;
	}

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join(output_name), serde_json::to_vec_pretty(&fixture)?)?;

	support::run_json_report_from(temp_dir)
}

fn single_work_continuity_job<'a>(report: &'a Value, job_id: &str) -> Result<&'a Value> {
	let jobs = support::array_at(report, "/jobs")?;

	support::find_by_field(jobs, "/job_id", job_id)
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
