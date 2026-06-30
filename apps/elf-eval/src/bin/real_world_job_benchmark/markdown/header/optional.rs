use crate::markdown::{
	KnowledgeSummary, MemorySummaryReport, ProactiveBriefSummaryReport, ReportSummary,
	ScheduledMemorySummaryReport, WorkContinuitySummaryReport,
};

pub(in crate::markdown::header) fn render_markdown_optional_summary_metrics(
	out: &mut String,
	summary: &ReportSummary,
) {
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
