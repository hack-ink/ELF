use super::*;

pub(super) fn render_markdown_header(
	out: &mut String,
	report: &RealWorldReport,
	report_path: &str,
) {
	out.push_str("# Real-World Job Benchmark Report\n\n");
	out.push_str(
		"Goal: Publish a Markdown summary for one generated real_world_job benchmark report.\n",
	);
	out.push_str(
		"Read this when: You need a durable smoke report for real-world agent memory job fixtures.\n",
	);
	out.push_str(&format!("Inputs: `{}`.\n", md_inline(report_path)));
	out.push_str("Depends on: `apps/elf-eval/fixtures/`, `docs/spec/real_world_agent_memory_benchmark_v1.md`, and `Makefile.toml`.\n");
	out.push_str(
		"Verification: Compare this Markdown summary with the source JSON before committing.\n\n",
	);
	out.push_str("## Summary\n\n");
	out.push_str(&format!("- Run ID: `{}`\n", md_inline(report.run_id.as_str())));
	out.push_str(&format!("- Generated at: `{}`\n", md_inline(report.generated_at.as_str())));
	out.push_str(&format!("- Runner version: `{}`\n", md_inline(report.runner_version.as_str())));
	out.push_str(&format!("- Corpus profile: `{}`\n", md_inline(report.corpus_profile.as_str())));
	out.push_str(&format!(
		"- Adapter: `{}` ({})\n",
		md_inline(report.adapter.adapter_id.as_str()),
		md_inline(report.adapter.behavior.as_str())
	));
	out.push_str(&format!("- Jobs: `{}`\n", report.summary.job_count));
	out.push_str(&format!(
		"- Suites with encoded jobs: `{}`\n",
		report.summary.encoded_suite_count
	));
	out.push_str(&format!(
		"- Suites with `not_encoded` status: `{}`\n",
		report.not_encoded_suites.len()
	));
	out.push_str(&format!("- Status summary: `{}` pass, `{}` wrong_result, `{}` lifecycle_fail, `{}` incomplete, `{}` blocked, `{}` not_encoded, `{}` unsupported_claim\n", report.summary.pass, report.summary.wrong_result, report.summary.lifecycle_fail, report.summary.incomplete, report.summary.blocked, report.summary.not_encoded, report.summary.unsupported_claim));
	out.push_str(&format!(
		"- Unsupported claim count: `{}`\n",
		report.summary.unsupported_claim_count
	));
	out.push_str(&format!("- Wrong-result count: `{}`\n", report.summary.wrong_result_count));
	out.push_str(&format!("- Stale-answer count: `{}`\n", report.summary.stale_answer_count));
	out.push_str(&format!(
		"- Conflict detections: `{}`\n",
		report.summary.conflict_detection_count
	));
	out.push_str(&format!(
		"- Update rationales available: `{}`\n",
		report.summary.update_rationale_available_count
	));
	out.push_str(&format!(
		"- Temporal validity not encoded: `{}`\n",
		report.summary.temporal_validity_not_encoded_count
	));
	out.push_str(&format!(
		"- History readback encoded: `{}`\n",
		report.summary.history_readback_encoded_count
	));

	render_markdown_quality_summary(out, report);

	out.push_str(&format!("- Mean score: `{:.3}`\n", report.summary.mean_score));
	out.push_str(&format!(
		"- Mean latency: `{}`\n",
		optional_f64(report.summary.mean_latency_ms, " ms")
	));
	out.push_str(&format!("- Cost: `{}`\n", cost_display(report.summary.total_cost.as_ref())));
	out.push_str(&format!(
		"- Operator-debug jobs: `{}`\n",
		report.summary.operator_debug_job_count
	));
	out.push_str(&format!("- Raw SQL needed: `{}`\n", report.summary.raw_sql_needed_count));
	out.push_str(&format!(
		"- Trace-incomplete debug jobs: `{}`\n",
		report.summary.trace_incomplete_count
	));
	out.push_str(&format!("- Operator UX gaps: `{}`\n", report.summary.operator_ux_gap_count));

	render_markdown_optional_summary_metrics(out, &report.summary);

	out.push_str(&format!(
		"- Private corpus redaction: `{}`\n\n",
		md_inline(report.private_corpus_redaction.policy.as_str())
	));
}

fn render_markdown_optional_summary_metrics(out: &mut String, summary: &ReportSummary) {
	if let Some(knowledge) = &summary.knowledge {
		render_markdown_knowledge_summary_metrics(out, knowledge);
	}
	if let Some(memory_summary) = &summary.memory_summary {
		render_markdown_memory_summary_metrics(out, memory_summary);
	}
	if let Some(proactive) = &summary.proactive_brief {
		render_markdown_proactive_summary_metrics(out, proactive);
	}
	if let Some(scheduled) = &summary.scheduled_memory {
		render_markdown_scheduled_summary_metrics(out, scheduled);
	}
	if let Some(work_continuity) = &summary.work_continuity {
		render_markdown_work_continuity_summary_metrics(out, work_continuity);
	}
}

fn render_markdown_knowledge_summary_metrics(out: &mut String, knowledge: &KnowledgeSummary) {
	out.push_str(&format!("- Knowledge citation coverage: `{:.3}`\n", knowledge.citation_coverage));
	out.push_str(&format!("- Stale claim detection: `{:.3}`\n", knowledge.stale_claim_detection));
	out.push_str(&format!("- Rebuild determinism: `{:.3}`\n", knowledge.rebuild_determinism));
	out.push_str(&format!(
		"- Backlinks: `{}` total, `{:.3}` page coverage\n",
		knowledge.backlink_count, knowledge.backlink_coverage
	));
	out.push_str(&format!("- Version diff coverage: `{:.3}`\n", knowledge.version_diff_coverage));
	out.push_str(&format!("- Page usefulness: `{:.3}`\n", knowledge.page_usefulness));
	out.push_str(&format!(
		"- Unsupported summary count: `{}`\n",
		knowledge.unsupported_summary_count
	));
}

fn render_markdown_memory_summary_metrics(out: &mut String, memory_summary: &MemorySummaryReport) {
	out.push_str(&format!(
		"- Memory summary entries: `{}` across `{}` artifact(s)\n",
		memory_summary.entry_count, memory_summary.summary_count
	));
	out.push_str(&format!(
		"- Memory summary source-ref coverage: `{}/{}` (`{:.3}`)\n",
		memory_summary.source_ref_entry_count,
		memory_summary.source_ref_required_count,
		memory_summary.source_ref_coverage
	));
	out.push_str(&format!(
		"- Memory summary invalid top-of-mind count: `{}`\n",
		memory_summary.invalid_top_of_mind_count
	));
	out.push_str(&format!(
		"- Memory summary unsupported derived entries: `{}`\n",
		memory_summary.unsupported_derived_entry_count
	));
	out.push_str(&format!(
		"- Memory summary unsupported current entries: `{}`\n",
		memory_summary.unsupported_current_entry_count
	));
}

fn render_markdown_proactive_summary_metrics(
	out: &mut String,
	proactive: &ProactiveBriefSummaryReport,
) {
	out.push_str(&format!(
		"- Proactive brief suggestions: `{}` across `{}` artifact(s)\n",
		proactive.suggestion_count, proactive.brief_count
	));
	out.push_str(&format!(
		"- Proactive evidence-ref coverage: `{}/{}` (`{:.3}`)\n",
		proactive.evidence_ref_suggestion_count,
		proactive.evidence_ref_required_count,
		proactive.evidence_ref_coverage
	));
	out.push_str(&format!(
		"- Proactive freshness/action rationale coverage: `{:.3}` / `{:.3}`\n",
		proactive.freshness_coverage, proactive.action_rationale_coverage
	));
	out.push_str(&format!(
		"- Proactive stale/currentness violations: `{}` invalid current, `{}` tombstone violation(s)\n",
		proactive.invalid_current_suggestion_count, proactive.tombstone_violation_count
	));
	out.push_str(&format!(
		"- Proactive rejected/deferred suggestions: `{}` rejected, `{}` deferred\n",
		proactive.rejected_count, proactive.deferred_count
	));
}

fn render_markdown_scheduled_summary_metrics(
	out: &mut String,
	scheduled: &ScheduledMemorySummaryReport,
) {
	out.push_str(&format!(
		"- Scheduled memory outputs: `{}` across `{}` task run(s)\n",
		scheduled.output_count, scheduled.task_run_count
	));
	out.push_str(&format!(
		"- Scheduled memory evidence-ref coverage: `{}/{}` (`{:.3}`)\n",
		scheduled.evidence_ref_output_count,
		scheduled.evidence_ref_required_count,
		scheduled.evidence_ref_coverage
	));
	out.push_str(&format!(
		"- Scheduled memory freshness/action/trace coverage: `{:.3}` / `{:.3}` / `{:.3}`\n",
		scheduled.freshness_coverage, scheduled.action_rationale_coverage, scheduled.trace_coverage
	));
	out.push_str(&format!(
		"- Scheduled memory stale/currentness violations: `{}` invalid current, `{}` tombstone violation(s)\n",
		scheduled.invalid_current_output_count, scheduled.tombstone_violation_count
	));
	out.push_str(&format!(
		"- Scheduled memory source mutations: `{}`\n",
		scheduled.source_mutation_count
	));
}

fn render_markdown_work_continuity_summary_metrics(
	out: &mut String,
	work_continuity: &WorkContinuitySummaryReport,
) {
	out.push_str(&format!(
		"- Work continuity readbacks: `{}` entries across `{}` artifact(s)\n",
		work_continuity.entry_count, work_continuity.readback_count
	));
	out.push_str(&format!(
		"- Work continuity reset/resume and rationale recall: `{:.3}` / `{:.3}`\n",
		work_continuity.reset_resume_success_rate, work_continuity.decision_rationale_recall_rate
	));
	out.push_str(&format!(
		"- Work continuity rejected-option suppression and explicit next-step precision: `{:.3}` / `{:.3}`\n",
		work_continuity.rejected_option_suppression_rate,
		work_continuity.explicit_next_step_precision
	));
	out.push_str(&format!(
		"- Work continuity inferred-step labeling and handoff source-ref coverage: `{:.3}` / `{:.3}`\n",
		work_continuity.inferred_next_step_labeling_rate,
		work_continuity.handoff_source_ref_coverage
	));
	out.push_str(&format!(
		"- Work continuity redaction and janitor false-promotion rates: `{:.3}` / `{:.3}`\n",
		work_continuity.redaction_rate, work_continuity.janitor_false_promotion_rate
	));
	out.push_str(&format!(
		"- Work continuity hard-fail markers: `{}` sensitive persistence, `{}` rejected resurrection, `{}` inferred instructions, `{}` journal-only authority claim(s)\n",
		work_continuity.sensitive_marker_persistence_count,
		work_continuity.rejected_option_resurrection_count,
		work_continuity.inferred_step_instruction_count,
		work_continuity.journal_only_authority_claim_count
	));
}

fn render_markdown_quality_summary(out: &mut String, report: &RealWorldReport) {
	out.push_str(&format!(
		"- Evidence coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.evidence_covered_count,
		report.summary.evidence_required_count,
		report.summary.evidence_coverage
	));
	out.push_str(&format!(
		"- Source-ref coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.source_ref_covered_count,
		report.summary.source_ref_required_count,
		report.summary.source_ref_coverage
	));
	out.push_str(&format!(
		"- Quote coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.quote_covered_count,
		report.summary.quote_required_count,
		report.summary.quote_coverage
	));
	out.push_str(&format!("- Stale retrieval count: `{}`\n", report.summary.stale_retrieval_count));
	out.push_str(&format!(
		"- Scope correctness: `{}/{}` (`{:.3}`), violations `{}`\n",
		report.summary.scope_correct_count,
		report.summary.scope_check_count,
		report.summary.scope_correctness,
		report.summary.scope_violation_count
	));
	out.push_str(&format!("- Redaction leak count: `{}`\n", report.summary.redaction_leak_count));
	out.push_str(&format!(
		"- Qdrant rebuild cases: `{}` encoded, `{}` pass\n",
		report.summary.qdrant_rebuild_case_count, report.summary.qdrant_rebuild_pass_count
	));
	out.push_str(&format!(
		"- Expected evidence recall: `{:.3}` ({}/{})\n",
		report.summary.expected_evidence_recall,
		report.summary.expected_evidence_matched,
		report.summary.expected_evidence_total
	));
	out.push_str(&format!(
		"- Irrelevant context ratio: `{:.3}` ({} irrelevant)\n",
		report.summary.irrelevant_context_ratio, report.summary.irrelevant_context_count
	));
	out.push_str(&format!(
		"- Trace explainability: `{}` job(s), `{}` wrong-result stage attribution(s)\n",
		report.summary.trace_explainability_count,
		report.summary.wrong_result_stage_attribution_count
	));
	out.push_str(&format!(
		"- Consolidation source mutation count: `{}`\n",
		report.summary.consolidation.source_mutation_count
	));
}
