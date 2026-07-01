use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn work_continuity_fixtures_score_required_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::work_continuity_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));

	assert_work_continuity_summary_counts(&report);

	let suites = support::array_at(&report, "/suites")?;
	let work_continuity = support::find_by_field(suites, "/suite_id", "work_continuity")?;

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
