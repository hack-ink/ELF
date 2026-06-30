use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn runner_discovers_nested_fixture_layout() -> Result<()> {
	let report = support::run_json_report_from(support::fixture_root())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(82));

	Ok(())
}

#[test]
fn operator_debug_fixture_reports_trace_links_and_failure_details() -> Result<()> {
	let report = support::run_json_report_from(support::operator_debug_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(7));
	assert_eq!(
		report.pointer("/summary/operator_debug_job_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(report.pointer("/summary/raw_sql_needed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/trace_incomplete_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/operator_ux_gap_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(7));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(3)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let dropped = support::find_by_field(jobs, "/job_id", "operator-debug-dropped-evidence-001")?;
	let selected =
		support::find_by_field(jobs, "/job_id", "operator-debug-selected-not-narrated-001")?;
	let compact =
		support::find_by_field(jobs, "/job_id", "operator-debug-qmd-style-compact-replay-001")?;

	assert_eq!(dropped.pointer("/status").and_then(Value::as_str), Some("pass"));
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
	assert_eq!(
		dropped.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("filter.read_profile")
	);
	assert!(support::array_contains_str(
		dropped,
		"/trace_explainability/stages/1/dropped_evidence",
		"trace-dropped-expected"
	)?);
	assert!(support::array_contains_str(
		dropped,
		"/trace_explainability/stages/1/distractor_evidence",
		"trace-dropped-decoy"
	)?);
	assert!(support::array_contains_str(dropped, "/produced_evidence", "trace-dropped-expected")?);
	assert_eq!(selected.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		selected.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("selection.narration")
	);
	assert_eq!(
		selected.pointer("/operator_debug/failure_mode").and_then(Value::as_str),
		Some("selected_but_not_narrated")
	);
	assert_eq!(compact.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		compact.pointer("/operator_debug/failure_mode").and_then(Value::as_str),
		Some("qmd_style_compact_replay")
	);
	assert_eq!(
		compact.pointer("/operator_debug/replay_command_available").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		compact.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("recall_debug.compact_replay")
	);
	assert!(support::array_contains_str(
		compact,
		"/trace_explainability/stages/4/kept_evidence",
		"compact-replay-artifact"
	)?);
	assert!(support::array_contains_str(
		compact,
		"/produced_evidence",
		"qmd-short-replay-reference"
	)?);

	Ok(())
}

#[test]
fn consolidation_fixtures_report_reviewable_proposal_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::consolidation_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(
		report.pointer("/summary/consolidation/proposal_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/consolidation/proposal_unsupported_claim_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/executable_gap_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/lineage_completeness").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/review_action_correctness").and_then(Value::as_f64),
		Some(1.0)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let project_summary =
		support::find_by_field(jobs, "/job_id", "consolidation-project-summary-apply-001")?;
	let contradiction =
		support::find_by_field(jobs, "/job_id", "consolidation-contradiction-report-discard-001")?;

	assert_eq!(
		project_summary
			.pointer("/consolidation/proposals/0/actual_review_action")
			.and_then(Value::as_str),
		Some("apply")
	);
	assert_eq!(
		contradiction
			.pointer("/consolidation/proposals/0/actual_review_action")
			.and_then(Value::as_str),
		Some("discard")
	);
	assert_eq!(
		contradiction
			.pointer("/consolidation/proposals/0/unsupported_claim_count")
			.and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(&report, "/suites")?;
	let consolidation_suite = support::find_by_field(suites, "/suite_id", "consolidation")?;

	assert_eq!(consolidation_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	Ok(())
}

#[test]
fn knowledge_fixtures_report_page_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::knowledge_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		report.pointer("/summary/knowledge/section_count").and_then(Value::as_u64),
		Some(13)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/citation_coverage").and_then(Value::as_f64),
		Some(0.923)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/rebuild_determinism").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_count").and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/pages_with_backlinks").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.979)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/pages_with_version_diff").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/allowed_variance_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(&report, "/suites")?;
	let knowledge_suite = support::find_by_field(suites, "/suite_id", "knowledge_compilation")?;

	assert_eq!(knowledge_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(knowledge_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = support::array_at(&report, "/jobs")?;
	let project_page_job = support::find_by_field(jobs, "/job_id", "knowledge-project-page-001")?;
	let watch_rebuild_job = support::find_by_field(jobs, "/job_id", "knowledge-watch-rebuild-003")?;

	assert_eq!(
		project_page_job.pointer("/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		project_page_job.pointer("/knowledge/untraced_section_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		watch_rebuild_job.pointer("/knowledge/pages_with_version_diff").and_then(Value::as_u64),
		Some(1)
	);
	assert!(
		watch_rebuild_job
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| answer
				.contains("PageIndex/OpenKB adapter claim as lint evidence")
				&& answer.contains("leaves source documents plus Memory Notes unmodified"))
	);

	Ok(())
}

#[test]
fn project_decisions_fixtures_report_decision_policy_cases() -> Result<()> {
	let report = support::run_json_report_from(support::project_decisions_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);

	let suites = support::array_at(&report, "/suites")?;
	let project_decisions = support::find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(project_decisions.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(project_decisions.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		project_decisions.pointer("/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let accepted =
		support::find_by_field(jobs, "/job_id", "project-decision-accepted-typed-failures-001")?;
	let reversal =
		support::find_by_field(jobs, "/job_id", "project-decision-reversal-live-baseline-001")?;
	let validation =
		support::find_by_field(jobs, "/job_id", "project-decision-current-validation-gate-001")?;
	let tradeoff =
		support::find_by_field(jobs, "/job_id", "project-decision-tradeoff-fixture-backed-001")?;
	let caveat =
		support::find_by_field(jobs, "/job_id", "project-decision-private-manifest-caveat-001")?;

	assert_eq!(accepted.pointer("/answer_type").and_then(Value::as_str), Some("decision_record"));
	assert_eq!(
		accepted.pointer("/expected_evidence").and_then(Value::as_array).map(Vec::len),
		Some(2)
	);
	assert_eq!(
		reversal.pointer("/evolution/historical_evidence/0").and_then(Value::as_str),
		Some("live-baseline-suite-win-old")
	);
	assert_eq!(
		validation.pointer("/evolution/current_evidence/0").and_then(Value::as_str),
		Some("validation-gate-current-decodex")
	);
	assert_eq!(tradeoff.pointer("/requires_caveat").and_then(Value::as_bool), Some(true));
	assert_eq!(caveat.pointer("/can_answer_unknown").and_then(Value::as_bool), Some(true));

	for job in jobs {
		let expected_evidence = support::array_at(job, "/expected_evidence")?;

		assert!(
			!expected_evidence.is_empty(),
			"project decision job {} must declare required evidence",
			job.pointer("/job_id").and_then(Value::as_str).unwrap_or("<unknown>")
		);
	}
	for entry in fs::read_dir(support::project_decisions_fixture_dir())? {
		let path = entry?.path();

		if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
			continue;
		}

		let fixture = serde_json::from_str::<Value>(&fs::read_to_string(path)?)?;
		let required_evidence = support::array_at(&fixture, "/required_evidence")?;
		let negative_traps = support::array_at(&fixture, "/negative_traps")?;

		assert!(!required_evidence.is_empty());
		assert!(!negative_traps.is_empty());
	}

	Ok(())
}
