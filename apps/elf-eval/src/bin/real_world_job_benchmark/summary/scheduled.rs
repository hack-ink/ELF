use crate::summary::{self, JobReport, ScheduledMemorySummaryReport};

pub(super) fn scheduled_memory_summary_impl(
	jobs: &[JobReport],
) -> Option<ScheduledMemorySummaryReport> {
	let scheduled_jobs =
		jobs.iter().filter_map(|job| job.scheduled_memory.as_ref()).collect::<Vec<_>>();

	if scheduled_jobs.is_empty() {
		return None;
	}

	let job_count = scheduled_jobs.len();
	let output_count = scheduled_jobs.iter().map(|metrics| metrics.output_count).sum::<usize>();
	let evidence_ref_required_count =
		scheduled_jobs.iter().map(|metrics| metrics.evidence_ref_required_count).sum();
	let evidence_ref_output_count =
		scheduled_jobs.iter().map(|metrics| metrics.evidence_ref_output_count).sum();
	let freshness_marker_count =
		scheduled_jobs.iter().map(|metrics| metrics.freshness_marker_count).sum();
	let action_rationale_count =
		scheduled_jobs.iter().map(|metrics| metrics.action_rationale_count).sum();
	let trace_required_count =
		scheduled_jobs.iter().map(|metrics| metrics.trace_required_count).sum();
	let trace_complete_count =
		scheduled_jobs.iter().map(|metrics| metrics.trace_complete_count).sum();

	Some(ScheduledMemorySummaryReport {
		job_count,
		task_run_count: scheduled_jobs.iter().map(|metrics| metrics.task_run_count).sum(),
		output_count,
		required_task_kind_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.required_task_kind_count)
			.sum(),
		covered_required_task_kind_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.covered_required_task_kind_count)
			.sum(),
		missing_required_task_kind_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.missing_required_task_kind_count)
			.sum(),
		evidence_ref_required_count,
		evidence_ref_output_count,
		evidence_ref_coverage: summary::ratio(
			evidence_ref_output_count,
			evidence_ref_required_count,
		),
		freshness_marker_count,
		freshness_coverage: summary::ratio(freshness_marker_count, output_count),
		action_rationale_count,
		action_rationale_coverage: summary::ratio(action_rationale_count, output_count),
		trace_required_count,
		trace_complete_count,
		trace_coverage: summary::ratio(trace_complete_count, trace_required_count),
		source_mutation_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.source_mutation_count)
			.sum(),
		current_output_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.current_output_count)
			.sum(),
		non_current_output_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.non_current_output_count)
			.sum(),
		invalid_current_output_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.invalid_current_output_count)
			.sum(),
		untraced_output_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.untraced_output_count)
			.sum(),
		unsupported_current_output_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.unsupported_current_output_count)
			.sum(),
		tombstone_violation_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.tombstone_violation_count)
			.sum(),
		source_trace_selected_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.source_trace_selected_count)
			.sum(),
		source_trace_dropped_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.source_trace_dropped_count)
			.sum(),
		source_trace_stale_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.source_trace_stale_count)
			.sum(),
		source_trace_superseded_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.source_trace_superseded_count)
			.sum(),
		source_trace_tombstone_count: scheduled_jobs
			.iter()
			.map(|metrics| metrics.source_trace_tombstone_count)
			.sum(),
	})
}
