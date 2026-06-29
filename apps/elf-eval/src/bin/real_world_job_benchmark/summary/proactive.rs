use crate::summary::{self, JobReport, ProactiveBriefSummaryReport};

pub(super) fn proactive_brief_summary_impl(
	jobs: &[JobReport],
) -> Option<ProactiveBriefSummaryReport> {
	let proactive_jobs =
		jobs.iter().filter_map(|job| job.proactive_brief.as_ref()).collect::<Vec<_>>();

	if proactive_jobs.is_empty() {
		return None;
	}

	let job_count = proactive_jobs.len();
	let suggestion_count =
		proactive_jobs.iter().map(|metrics| metrics.suggestion_count).sum::<usize>();
	let evidence_ref_required_count =
		proactive_jobs.iter().map(|metrics| metrics.evidence_ref_required_count).sum();
	let evidence_ref_suggestion_count =
		proactive_jobs.iter().map(|metrics| metrics.evidence_ref_suggestion_count).sum();
	let freshness_marker_count =
		proactive_jobs.iter().map(|metrics| metrics.freshness_marker_count).sum();
	let action_rationale_count =
		proactive_jobs.iter().map(|metrics| metrics.action_rationale_count).sum();

	Some(ProactiveBriefSummaryReport {
		job_count,
		brief_count: proactive_jobs.iter().map(|metrics| metrics.brief_count).sum(),
		suggestion_count,
		required_suggestion_kind_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.required_suggestion_kind_count)
			.sum(),
		covered_required_suggestion_kind_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.covered_required_suggestion_kind_count)
			.sum(),
		missing_required_suggestion_kind_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.missing_required_suggestion_kind_count)
			.sum(),
		evidence_ref_required_count,
		evidence_ref_suggestion_count,
		evidence_ref_coverage: summary::ratio(
			evidence_ref_suggestion_count,
			evidence_ref_required_count,
		),
		freshness_marker_count,
		freshness_coverage: summary::ratio(freshness_marker_count, suggestion_count),
		action_rationale_count,
		action_rationale_coverage: summary::ratio(action_rationale_count, suggestion_count),
		recommended_count: proactive_jobs.iter().map(|metrics| metrics.recommended_count).sum(),
		deferred_count: proactive_jobs.iter().map(|metrics| metrics.deferred_count).sum(),
		rejected_count: proactive_jobs.iter().map(|metrics| metrics.rejected_count).sum(),
		current_suggestion_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.current_suggestion_count)
			.sum(),
		non_current_suggestion_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.non_current_suggestion_count)
			.sum(),
		stale_warning_count: proactive_jobs.iter().map(|metrics| metrics.stale_warning_count).sum(),
		invalid_current_suggestion_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.invalid_current_suggestion_count)
			.sum(),
		untraced_suggestion_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.untraced_suggestion_count)
			.sum(),
		unsupported_current_suggestion_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.unsupported_current_suggestion_count)
			.sum(),
		tombstone_violation_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.tombstone_violation_count)
			.sum(),
		source_trace_selected_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.source_trace_selected_count)
			.sum(),
		source_trace_dropped_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.source_trace_dropped_count)
			.sum(),
		source_trace_stale_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.source_trace_stale_count)
			.sum(),
		source_trace_superseded_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.source_trace_superseded_count)
			.sum(),
		source_trace_tombstone_count: proactive_jobs
			.iter()
			.map(|metrics| metrics.source_trace_tombstone_count)
			.sum(),
	})
}
