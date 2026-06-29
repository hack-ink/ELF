use color_eyre::Result;
use serde_json::Value;

use crate::support;

fn assert_root_knowledge_summary(report: &Value) {
	assert_eq!(report.pointer("/summary/knowledge/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.979)
	);
}

fn assert_root_aggregate_summary(report: &Value) -> Result<()> {
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

	super::assert_tracked_external_blocker_row(pageindex, "VectifyAI PageIndex", true)?;
	super::assert_tracked_external_blocker_row(openkb, "VectifyAI OpenKB", true)?;
	super::assert_tracked_external_blocker_row(honcho, "plastic-labs Honcho", false)?;

	Ok(())
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

fn assert_root_aggregate_suites(report: &Value) -> Result<()> {
	let suites = support::array_at(report, "/suites")?;

	for suite_id in [
		"trust_source_of_truth",
		"work_resume",
		"project_decisions",
		"retrieval",
		"capture_integration",
		"personalization",
		"consolidation",
		"memory_summary",
		"knowledge_compilation",
		"operator_debugging_ux",
		"memory_evolution",
		"adversarial_quality",
		"core_archival_memory",
		"work_continuity",
	] {
		let suite = support::find_by_field(suites, "/suite_id", suite_id)?;

		assert_eq!(suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	}

	let memory_evolution = support::find_by_field(suites, "/suite_id", "memory_evolution")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));

	let project_decisions = support::find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(project_decisions.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		project_decisions.pointer("/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);

	let debug_suite = support::find_by_field(suites, "/suite_id", "operator_debugging_ux")?;

	assert_eq!(debug_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	let core_suite = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let adversarial = support::find_by_field(suites, "/suite_id", "adversarial_quality")?;

	assert_eq!(adversarial.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(adversarial.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let production_ops = support::find_by_field(suites, "/suite_id", "production_ops")?;

	assert_eq!(production_ops.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(production_ops.pointer("/encoded_job_count").and_then(Value::as_u64), Some(8));

	let proactive = support::find_by_field(suites, "/suite_id", "proactive_brief")?;

	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(proactive.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let scheduled = support::find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let source_library = support::find_by_field(suites, "/suite_id", "source_library")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/encoded_job_count").and_then(Value::as_u64), Some(2));

	let context_trajectory = support::find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(context_trajectory.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(context_trajectory.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let work_continuity = support::find_by_field(suites, "/suite_id", "work_continuity")?;

	assert_eq!(work_continuity.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(work_continuity.pointer("/encoded_job_count").and_then(Value::as_u64), Some(8));

	Ok(())
}

fn assert_root_aggregate_jobs(report: &Value) -> Result<()> {
	let jobs = support::array_at(report, "/jobs")?;
	let rebuild = support::find_by_field(jobs, "/job_id", "trust-sot-rebuild-001")?;
	let redaction = support::find_by_field(jobs, "/job_id", "capture-redaction-exclusion-001")?;
	let personalization =
		support::find_by_field(jobs, "/job_id", "personalization-scoped-preference-001")?;
	let relation_job =
		support::find_by_field(jobs, "/job_id", "memory-evolution-relation-temporal-001")?;
	let delete_job = support::find_by_field(jobs, "/job_id", "memory-evolution-delete-ttl-001")?;
	let stage_job =
		support::find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;
	let production_restore =
		support::find_by_field(jobs, "/job_id", "production-ops-restore-cold-start-001")?;
	let production_authority =
		support::find_by_field(jobs, "/job_id", "production-ops-authority-plane-recovery-001")?;
	let core_fallback =
		support::find_by_field(jobs, "/job_id", "core-archival-archival-fallback-001")?;
	let stale_core =
		support::find_by_field(jobs, "/job_id", "core-archival-stale-core-detection-001")?;
	let scheduled_weekly =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(rebuild.pointer("/qdrant_rebuild_case").and_then(Value::as_bool), Some(true));
	assert_eq!(
		production_restore.pointer("/qdrant_rebuild_case").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		production_authority.pointer("/qdrant_rebuild_case").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(production_authority.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		production_authority.pointer("/recovery_drills/0/contract_schema").and_then(Value::as_str),
		Some("elf.authority_recovery_drill/v1")
	);
	assert_eq!(redaction.pointer("/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(personalization.pointer("/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(personalization.pointer("/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(stage_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(relation_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(delete_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		delete_job.pointer("/evolution/selected_tombstone_evidence/0").and_then(Value::as_str),
		Some("delete-tombstone")
	);
	assert_eq!(
		delete_job.pointer("/evolution/selected_invalidation_evidence/0").and_then(Value::as_str),
		Some("delete-tombstone")
	);
	assert_eq!(core_fallback.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(stale_core.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(scheduled_weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		scheduled_weekly.pointer("/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		stage_job.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("rerank.score")
	);
	assert!(support::array_contains_str(stage_job, "/produced_evidence", "stage-target")?);

	Ok(())
}

#[test]
fn real_world_memory_fixtures_report_aggregate_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::real_world_memory_fixture_dir())?;

	assert_root_aggregate_summary(&report)?;
	assert_root_aggregate_suites(&report)?;
	assert_root_aggregate_jobs(&report)?;

	Ok(())
}
