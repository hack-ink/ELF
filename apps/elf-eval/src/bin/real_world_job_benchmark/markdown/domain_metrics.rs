use super::*;

pub(super) fn render_markdown_consolidation(out: &mut String, report: &RealWorldReport) {
	if report.summary.consolidation.proposal_count == 0 {
		return;
	}

	out.push_str("## Consolidation\n\n");
	out.push_str("| Job | Proposals | Usefulness | Lineage | Review Actions | Source Mutations | Proposal Unsupported Claims | Executable Gaps |\n");
	out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");

	for job in &report.jobs {
		let Some(consolidation) = &job.consolidation else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | `{}` | {} | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			consolidation.proposal_count,
			optional_f64(consolidation.proposal_usefulness, ""),
			optional_f64(consolidation.lineage_completeness, ""),
			optional_f64(consolidation.review_action_correctness, ""),
			consolidation.source_mutation_count,
			consolidation.proposal_unsupported_claim_count,
			consolidation.executable_gaps.len()
		));
	}

	out.push_str(
		"\nSource mutation count must remain `0` for proposal-only consolidation cases.\n\n",
	);

	render_markdown_consolidation_gaps(out, report);
}

pub(super) fn render_markdown_knowledge(out: &mut String, report: &RealWorldReport) {
	let knowledge_jobs =
		report.jobs.iter().filter(|job| job.knowledge.is_some()).collect::<Vec<_>>();

	if knowledge_jobs.is_empty() {
		return;
	}

	out.push_str("## Knowledge Page Metrics\n\n");
	out.push_str("| Job | Pages | Sections | Citation Coverage | Stale Claim Detection | Rebuild Determinism | Version Diff Coverage | Page Usefulness | Backlinks | Unsupported Summaries | Untraced Sections | Allowed Variance |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in knowledge_jobs {
		let Some(knowledge) = &job.knowledge else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{:.3}` | `{:.3}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			knowledge.page_count,
			knowledge.section_count,
			knowledge.citation_coverage,
			knowledge.stale_claim_detection,
			knowledge.rebuild_determinism,
			knowledge.version_diff_coverage,
			knowledge.page_usefulness,
			knowledge.backlink_count,
			knowledge.unsupported_summary_count,
			knowledge.untraced_section_count,
			knowledge.allowed_variance_count
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_memory_summary(out: &mut String, report: &RealWorldReport) {
	let memory_jobs =
		report.jobs.iter().filter(|job| job.memory_summary.is_some()).collect::<Vec<_>>();

	if memory_jobs.is_empty() {
		return;
	}

	out.push_str("## Memory Summary Metrics\n\n");
	out.push_str("| Job | Summaries | Entries | Categories | Source Coverage | Freshness | Rationale | Invalid Top-of-Mind | Untraced | Derived Unsupported | Unsupported Current | Tombstone Refs |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in memory_jobs {
		let Some(metrics) = &job.memory_summary else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{}/{}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			metrics.summary_count,
			metrics.entry_count,
			metrics.covered_required_category_count,
			metrics.required_category_count,
			metrics.source_ref_coverage,
			metrics.freshness_coverage,
			metrics.rationale_coverage,
			metrics.invalid_top_of_mind_count,
			metrics.untraced_entry_count,
			metrics.unsupported_derived_entry_count,
			metrics.unsupported_current_entry_count,
			metrics.tombstone_ref_count
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_proactive_brief(out: &mut String, report: &RealWorldReport) {
	let proactive_jobs =
		report.jobs.iter().filter(|job| job.proactive_brief.is_some()).collect::<Vec<_>>();

	if proactive_jobs.is_empty() {
		return;
	}

	out.push_str("## Proactive Brief Metrics\n\n");
	out.push_str("| Job | Briefs | Suggestions | Kinds | Evidence Coverage | Freshness | Action Rationale | Invalid Current | Untraced | Unsupported Current | Tombstone Violations | Rejected | Deferred |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in proactive_jobs {
		let Some(metrics) = &job.proactive_brief else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{}/{}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			metrics.brief_count,
			metrics.suggestion_count,
			metrics.covered_required_suggestion_kind_count,
			metrics.required_suggestion_kind_count,
			metrics.evidence_ref_coverage,
			metrics.freshness_coverage,
			metrics.action_rationale_coverage,
			metrics.invalid_current_suggestion_count,
			metrics.untraced_suggestion_count,
			metrics.unsupported_current_suggestion_count,
			metrics.tombstone_violation_count,
			metrics.rejected_count,
			metrics.deferred_count
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_scheduled_memory(out: &mut String, report: &RealWorldReport) {
	let scheduled_jobs =
		report.jobs.iter().filter(|job| job.scheduled_memory.is_some()).collect::<Vec<_>>();

	if scheduled_jobs.is_empty() {
		return;
	}

	out.push_str("## Scheduled Memory Metrics\n\n");
	out.push_str("| Job | Task Runs | Outputs | Kinds | Evidence Coverage | Freshness | Action Rationale | Trace Coverage | Invalid Current | Untraced | Unsupported Current | Tombstone Violations | Source Mutations |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in scheduled_jobs {
		let Some(metrics) = &job.scheduled_memory else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{}/{}` | `{:.3}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			metrics.task_run_count,
			metrics.output_count,
			metrics.covered_required_task_kind_count,
			metrics.required_task_kind_count,
			metrics.evidence_ref_coverage,
			metrics.freshness_coverage,
			metrics.action_rationale_coverage,
			metrics.trace_coverage,
			metrics.invalid_current_output_count,
			metrics.untraced_output_count,
			metrics.unsupported_current_output_count,
			metrics.tombstone_violation_count,
			metrics.source_mutation_count
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_work_continuity(out: &mut String, report: &RealWorldReport) {
	let work_jobs =
		report.jobs.iter().filter(|job| job.work_continuity.is_some()).collect::<Vec<_>>();

	if work_jobs.is_empty() {
		return;
	}

	out.push_str("## Work Continuity Metrics\n\n");
	out.push_str("| Job | Readbacks | Entries | Reset/Resume | Decision Rationale | Rejected Suppression | Explicit Precision | Inferred Labeling | Handoff Sources | Redaction | Janitor False Promotion | Sensitive Persistence | Journal Authority Claims |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in work_jobs {
		let Some(metrics) = &job.work_continuity else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{}/{}` (`{:.3}`) | `{}/{}` (`{:.3}`) | `{}/{}` (`{:.3}`) | `{}/{}` (`{:.3}`) | `{}/{}` (`{:.3}`) | `{}/{}` (`{:.3}`) | `{}/{}` (`{:.3}`) | `{}/{}` (`{:.3}`) | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			metrics.readback_count,
			metrics.entry_count,
			metrics.reset_resume_success_count,
			metrics.reset_resume_required_count,
			metrics.reset_resume_success_rate,
			metrics.decision_rationale_recalled_count,
			metrics.decision_rationale_required_count,
			metrics.decision_rationale_recall_rate,
			metrics.rejected_option_suppressed_count,
			metrics.rejected_option_required_count,
			metrics.rejected_option_suppression_rate,
			metrics.explicit_next_step_correct_count,
			metrics.explicit_next_step_returned_count,
			metrics.explicit_next_step_precision,
			metrics.inferred_next_step_labeled_count,
			metrics.inferred_next_step_required_count,
			metrics.inferred_next_step_labeling_rate,
			metrics.handoff_source_ref_covered_count,
			metrics.handoff_source_ref_required_count,
			metrics.handoff_source_ref_coverage,
			metrics.redaction_applied_count,
			metrics.redaction_required_count,
			metrics.redaction_rate,
			metrics.janitor_false_promotion_count,
			metrics.janitor_candidate_count,
			metrics.janitor_false_promotion_rate,
			metrics.sensitive_marker_persistence_count,
			metrics.journal_only_authority_claim_count
		));
	}

	out.push('\n');
}

fn render_markdown_consolidation_gaps(out: &mut String, report: &RealWorldReport) {
	let gaps = report
		.jobs
		.iter()
		.filter_map(|job| job.consolidation.as_ref().map(|consolidation| (job, consolidation)))
		.flat_map(|(job, consolidation)| {
			consolidation.executable_gaps.iter().map(move |gap| (job.job_id.as_str(), gap))
		})
		.collect::<Vec<_>>();

	if gaps.is_empty() {
		return;
	}

	out.push_str("### Executable Gaps\n\n");
	out.push_str("| Job | Primitive | Follow-Up Issue | Blocks Fixture Pass | Reason |\n");
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for (job_id, gap) in gaps {
		out.push_str(&format!(
			"| {} | {} | {} | `{}` | {} |\n",
			md_cell(job_id),
			md_cell(gap.primitive.as_str()),
			md_cell(gap.follow_up_issue.as_str()),
			gap.blocks_fixture_pass,
			md_cell(gap.reason.as_str())
		));
	}

	out.push('\n');
}
