use crate::markdown::{self, RealWorldReport};

pub(in crate::markdown) fn render_markdown_work_continuity(
	out: &mut String,
	report: &RealWorldReport,
) {
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
			markdown::md_cell(job.job_id.as_str()),
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
