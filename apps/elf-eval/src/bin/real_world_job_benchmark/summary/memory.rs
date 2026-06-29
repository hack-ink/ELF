use super::*;

pub(super) fn memory_summary_summary_impl(jobs: &[JobReport]) -> Option<MemorySummaryReport> {
	let memory_jobs = jobs.iter().filter_map(|job| job.memory_summary.as_ref()).collect::<Vec<_>>();

	if memory_jobs.is_empty() {
		return None;
	}

	let job_count = memory_jobs.len();
	let summary_count = memory_jobs.iter().map(|metrics| metrics.summary_count).sum();
	let entry_count = memory_jobs.iter().map(|metrics| metrics.entry_count).sum();
	let required_category_count =
		memory_jobs.iter().map(|metrics| metrics.required_category_count).sum();
	let covered_required_category_count =
		memory_jobs.iter().map(|metrics| metrics.covered_required_category_count).sum();
	let source_ref_required_count =
		memory_jobs.iter().map(|metrics| metrics.source_ref_required_count).sum();
	let source_ref_entry_count =
		memory_jobs.iter().map(|metrics| metrics.source_ref_entry_count).sum();
	let freshness_marker_count =
		memory_jobs.iter().map(|metrics| metrics.freshness_marker_count).sum();
	let rationale_count = memory_jobs.iter().map(|metrics| metrics.rationale_count).sum();

	Some(MemorySummaryReport {
		job_count,
		summary_count,
		entry_count,
		required_category_count,
		covered_required_category_count,
		missing_required_category_count: memory_jobs
			.iter()
			.map(|metrics| metrics.missing_required_category_count)
			.sum(),
		top_of_mind_count: memory_jobs.iter().map(|metrics| metrics.top_of_mind_count).sum(),
		background_count: memory_jobs.iter().map(|metrics| metrics.background_count).sum(),
		stale_count: memory_jobs.iter().map(|metrics| metrics.stale_count).sum(),
		superseded_count: memory_jobs.iter().map(|metrics| metrics.superseded_count).sum(),
		tombstone_count: memory_jobs.iter().map(|metrics| metrics.tombstone_count).sum(),
		derived_project_profile_count: memory_jobs
			.iter()
			.map(|metrics| metrics.derived_project_profile_count)
			.sum(),
		source_ref_required_count,
		source_ref_entry_count,
		source_ref_coverage: ratio(source_ref_entry_count, source_ref_required_count),
		freshness_marker_count,
		freshness_coverage: ratio(freshness_marker_count, entry_count),
		rationale_count,
		rationale_coverage: ratio(rationale_count, entry_count),
		invalid_top_of_mind_count: memory_jobs
			.iter()
			.map(|metrics| metrics.invalid_top_of_mind_count)
			.sum(),
		untraced_entry_count: memory_jobs.iter().map(|metrics| metrics.untraced_entry_count).sum(),
		derived_with_source_or_unsupported_count: memory_jobs
			.iter()
			.map(|metrics| metrics.derived_with_source_or_unsupported_count)
			.sum(),
		derived_missing_source_or_unsupported_count: memory_jobs
			.iter()
			.map(|metrics| metrics.derived_missing_source_or_unsupported_count)
			.sum(),
		unsupported_derived_entry_count: memory_jobs
			.iter()
			.map(|metrics| metrics.unsupported_derived_entry_count)
			.sum(),
		unsupported_current_entry_count: memory_jobs
			.iter()
			.map(|metrics| metrics.unsupported_current_entry_count)
			.sum(),
		tombstone_ref_count: memory_jobs.iter().map(|metrics| metrics.tombstone_ref_count).sum(),
		source_trace_selected_count: memory_jobs
			.iter()
			.map(|metrics| metrics.source_trace_selected_count)
			.sum(),
		source_trace_dropped_count: memory_jobs
			.iter()
			.map(|metrics| metrics.source_trace_dropped_count)
			.sum(),
		source_trace_stale_count: memory_jobs
			.iter()
			.map(|metrics| metrics.source_trace_stale_count)
			.sum(),
		source_trace_superseded_count: memory_jobs
			.iter()
			.map(|metrics| metrics.source_trace_superseded_count)
			.sum(),
		source_trace_tombstone_count: memory_jobs
			.iter()
			.map(|metrics| metrics.source_trace_tombstone_count)
			.sum(),
	})
}
