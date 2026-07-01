use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_root_aggregate_summary(report: &Value) -> Result<()> {
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(82));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(19));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(75));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/irrelevant_context_ratio").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(report.pointer("/summary/stale_retrieval_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(16)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/scope_violation_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_case_count").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_pass_count").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report.pointer("/summary/evidence_required_count").and_then(Value::as_u64),
		Some(180)
	);
	assert_eq!(
		report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64),
		Some(180)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/wrong_result_stage_attribution_count").and_then(Value::as_u64),
		Some(0)
	);

	assert_root_scoreboard_summary(report)?;

	assert_eq!(
		report.pointer("/summary/consolidation/proposal_count").and_then(Value::as_u64),
		Some(5)
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
		report.pointer("/summary/memory_summary/job_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);

	assert_root_knowledge_summary(report);
	assert_root_proactive_brief_summary(report);
	assert_root_scheduled_memory_summary(report);
	assert_root_work_continuity_summary(report);

	Ok(())
}

fn assert_root_scoreboard_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/scoreboard/summary_claim").and_then(Value::as_str),
		Some("typed_non_pass_present")
	);
	assert_eq!(
		report.pointer("/scoreboard/job_summary_claim").and_then(Value::as_str),
		Some("typed_non_pass_present")
	);
	assert_eq!(
		report.pointer("/scoreboard/job_typed_non_pass_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report.pointer("/scoreboard/external_adapter_typed_non_pass_count").and_then(Value::as_u64),
		Some(240)
	);
	assert_eq!(
		report.pointer("/scoreboard/typed_non_pass_count").and_then(Value::as_u64),
		Some(247)
	);
	assert_eq!(
		report.pointer("/scoreboard/unqualified_win_claim_allowed").and_then(Value::as_bool),
		Some(false)
	);
	assert!(support::array_contains_str(report, "/scoreboard/result_states", "not_comparable")?);
	assert_eq!(
		report.pointer("/scoreboard/metric_basis").and_then(Value::as_str),
		Some("produced_evidence_order")
	);
	assert_eq!(report.pointer("/scoreboard/retrieval_k").and_then(Value::as_u64), Some(5));

	assert_root_scoreboard_rows(report)?;

	for state in ["blocked", "incomplete", "not_encoded", "not_tested", "wrong_result"] {
		assert!(support::array_contains_str(
			report,
			"/scoreboard/typed_non_pass_states_present",
			state
		)?);
	}

	assert_eq!(
		support::string_array_at(report, "/scoreboard/job_typed_non_pass_states_present")?,
		["blocked"].map(str::to_owned)
	);

	for state in ["blocked", "incomplete", "not_encoded", "not_tested", "wrong_result"] {
		assert!(support::array_contains_str(
			report,
			"/scoreboard/external_adapter_typed_non_pass_states_present",
			state
		)?);
	}

	Ok(())
}

fn assert_root_scoreboard_rows(report: &Value) -> Result<()> {
	let rows = support::array_at(report, "/scoreboard/rows")?;
	let elf = support::find_by_field(rows, "/product_id", "elf_current_report")?;
	let qmd = support::find_by_field(rows, "/product_id", "qmd")?;
	let graphify = support::find_by_field(rows, "/product_id", "graphify")?;
	let pageindex = support::find_by_field(rows, "/product_id", "vectifyai_pageindex")?;
	let openkb = support::find_by_field(rows, "/product_id", "vectifyai_openkb")?;
	let honcho = support::find_by_field(rows, "/product_id", "plastic_labs_honcho")?;

	assert_eq!(rows.len(), 20);
	assert_eq!(elf.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert_eq!(elf.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
	assert_eq!(elf.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/same_corpus").and_then(Value::as_bool), Some(true));
	assert_eq!(elf.pointer("/source_id_mapped").and_then(Value::as_bool), Some(true));
	assert_eq!(elf.pointer("/product_runtime").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/metrics/retrieval/recall_at_k").and_then(Value::as_f64), Some(0.988));
	assert_eq!(
		elf.pointer("/metrics/retrieval/precision_at_k").and_then(Value::as_f64),
		Some(0.415)
	);
	assert_eq!(elf.pointer("/metrics/retrieval/mrr").and_then(Value::as_f64), Some(0.988));
	assert_eq!(elf.pointer("/metrics/retrieval/ndcg").and_then(Value::as_f64), Some(0.985));
	assert_eq!(
		elf.pointer("/metrics/lifecycle/stale_suppression").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		elf.pointer("/metrics/lifecycle/update_correctness").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		elf.pointer("/metrics/lifecycle/delete_correctness").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		elf.pointer("/metrics/coverage/typed_non_pass_count").and_then(Value::as_u64),
		Some(7)
	);
	assert!(support::array_contains_str(
		elf,
		"/next_evidence",
		"Run a Docker-contained product-runtime adapter for this row."
	)?);

	for competitor in [qmd, graphify] {
		assert_eq!(
			competitor.pointer("/evidence_class").and_then(Value::as_str),
			Some("live_real_world")
		);
		assert_eq!(
			competitor.pointer("/result_state").and_then(Value::as_str),
			Some("wrong_result")
		);
		assert_eq!(competitor.pointer("/product_runtime").and_then(Value::as_bool), Some(true));
		assert_eq!(
			competitor.pointer("/container_digest_identified").and_then(Value::as_bool),
			Some(false)
		);
		assert!(competitor.pointer("/metrics/retrieval/recall_at_k").is_some_and(Value::is_null));
		assert!(support::array_contains_str(
			competitor,
			"/next_evidence",
			"Record container image digest evidence."
		)?);
	}

	crate::assert_tracked_external_blocker_row(pageindex, "VectifyAI PageIndex", true)?;
	crate::assert_tracked_external_blocker_row(openkb, "VectifyAI OpenKB", true)?;
	crate::assert_tracked_external_blocker_row(honcho, "plastic-labs Honcho", false)?;

	Ok(())
}

fn assert_root_knowledge_summary(report: &Value) {
	assert_eq!(report.pointer("/summary/knowledge/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.979)
	);
}

fn assert_root_proactive_brief_summary(report: &Value) {
	assert_eq!(
		report.pointer("/summary/proactive_brief/job_count").and_then(Value::as_u64),
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
}

fn assert_root_scheduled_memory_summary(report: &Value) {
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
}

fn assert_root_work_continuity_summary(report: &Value) {
	assert_eq!(
		report.pointer("/summary/work_continuity/job_count").and_then(Value::as_u64),
		Some(8)
	);
	assert_eq!(
		report
			.pointer("/summary/work_continuity/reset_resume_success_rate")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/work_continuity/decision_rationale_recall_rate")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/work_continuity/rejected_option_suppression_rate")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/work_continuity/inferred_step_instruction_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/work_continuity/sensitive_marker_persistence_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/work_continuity/janitor_false_promotion_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/work_continuity/journal_only_authority_claim_count")
			.and_then(Value::as_u64),
		Some(0)
	);
}
