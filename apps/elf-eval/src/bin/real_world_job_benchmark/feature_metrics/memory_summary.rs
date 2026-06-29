use crate::feature_metrics::{
	self, BTreeSet, MemorySummaryArtifact, MemorySummaryEntry, MemorySummaryJobMetrics,
	MemorySummarySourceTrace, ProducedAnswer, RealWorldJob, UnsupportedClaimReport,
};

pub(super) fn memory_summary_metrics_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<MemorySummaryJobMetrics> {
	if answer.memory_summaries.is_empty() {
		return None;
	}

	let mut metrics = MemorySummaryJobMetrics {
		summary_count: answer.memory_summaries.len(),
		required_category_count: job
			.memory_summary
			.as_ref()
			.map_or(0, |summary| summary.required_categories.len()),
		..MemorySummaryJobMetrics::default()
	};
	let mut categories = BTreeSet::new();

	for summary in &answer.memory_summaries {
		accumulate_memory_summary_metrics(summary, &mut metrics, &mut categories);
	}

	let covered_required_category_count = job.memory_summary.as_ref().map_or(0, |summary| {
		summary.required_categories.iter().filter(|category| categories.contains(*category)).count()
	});

	metrics.covered_required_category_count = covered_required_category_count;
	metrics.missing_required_category_count =
		metrics.required_category_count.saturating_sub(covered_required_category_count);
	metrics.source_ref_coverage =
		feature_metrics::ratio(metrics.source_ref_entry_count, metrics.source_ref_required_count);
	metrics.freshness_coverage =
		feature_metrics::ratio(metrics.freshness_marker_count, metrics.entry_count);
	metrics.rationale_coverage =
		feature_metrics::ratio(metrics.rationale_count, metrics.entry_count);

	Some(metrics)
}

pub(super) fn memory_summary_non_current_trace_refs_impl(
	trace: &MemorySummarySourceTrace,
) -> BTreeSet<&str> {
	trace
		.stale_source_refs
		.iter()
		.chain(trace.superseded_source_refs.iter())
		.chain(trace.tombstone_source_refs.iter())
		.map(|item| item.evidence_id.as_str())
		.collect()
}

pub(super) fn unsupported_memory_summary_claims_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Vec<UnsupportedClaimReport> {
	answer
		.memory_summaries
		.iter()
		.flat_map(|summary| {
			summary.entries.iter().filter_map(|entry| {
				if entry.category != "derived_project_profile"
					|| !entry.source_refs.is_empty()
					|| !entry.unsupported_claim_flags.is_empty()
				{
					return None;
				}

				Some(UnsupportedClaimReport {
					suite_id: job.suite.clone(),
					job_id: job.job_id.clone(),
					claim_id: Some(format!("{}:{}", summary.summary_id, entry.entry_id)),
					claim_text: feature_metrics::bounded_text(entry.text.as_str(), 240),
					reason:
						"derived memory summary entry has no source refs and no unsupported-claim flags"
							.to_string(),
					evidence_ids: entry.source_refs.clone(),
				})
			})
		})
		.collect()
}

fn accumulate_memory_summary_metrics(
	summary: &MemorySummaryArtifact,
	metrics: &mut MemorySummaryJobMetrics,
	categories: &mut BTreeSet<String>,
) {
	metrics.source_trace_selected_count += summary.source_trace.selected_source_refs.len();
	metrics.source_trace_dropped_count += summary.source_trace.dropped_source_refs.len();
	metrics.source_trace_stale_count += summary.source_trace.stale_source_refs.len();
	metrics.source_trace_superseded_count += summary.source_trace.superseded_source_refs.len();
	metrics.source_trace_tombstone_count += summary.source_trace.tombstone_source_refs.len();

	let non_current_source_refs = memory_summary_non_current_trace_refs_impl(&summary.source_trace);

	for entry in &summary.entries {
		metrics.entry_count += 1;

		categories.insert(entry.category.clone());

		accumulate_memory_summary_category(entry.category.as_str(), metrics);

		if memory_summary_entry_requires_source_ref(entry) {
			metrics.source_ref_required_count += 1;

			if entry.source_refs.is_empty() {
				metrics.untraced_entry_count += 1;
			}
		}
		if !entry.source_refs.is_empty() {
			metrics.source_ref_entry_count += 1;
		}
		if memory_summary_entry_has_freshness(entry) {
			metrics.freshness_marker_count += 1;
		}
		if memory_summary_entry_has_rationale(entry) {
			metrics.rationale_count += 1;
		}
		if memory_summary_entry_is_invalid_top_of_mind(entry, &non_current_source_refs) {
			metrics.invalid_top_of_mind_count += 1;
		}
		if entry.category == "derived_project_profile" {
			let has_support =
				!entry.source_refs.is_empty() || !entry.unsupported_claim_flags.is_empty();

			if has_support {
				metrics.derived_with_source_or_unsupported_count += 1;
			} else {
				metrics.derived_missing_source_or_unsupported_count += 1;
			}
			if !entry.unsupported_claim_flags.is_empty() {
				metrics.unsupported_derived_entry_count += 1;
			}
			if memory_summary_entry_includes_unsupported_current_claim(entry) {
				metrics.unsupported_current_entry_count += 1;
			}
		}

		metrics.tombstone_ref_count += entry.freshness.tombstone_refs.len();
	}
}

fn accumulate_memory_summary_category(category: &str, metrics: &mut MemorySummaryJobMetrics) {
	match category {
		"top_of_mind" => metrics.top_of_mind_count += 1,
		"background" => metrics.background_count += 1,
		"stale" => metrics.stale_count += 1,
		"superseded" => metrics.superseded_count += 1,
		"tombstone" => metrics.tombstone_count += 1,
		"derived_project_profile" => metrics.derived_project_profile_count += 1,
		_ => {},
	}
}

fn memory_summary_entry_requires_source_ref(entry: &MemorySummaryEntry) -> bool {
	!(entry.category == "derived_project_profile"
		&& entry.source_refs.is_empty()
		&& !entry.unsupported_claim_flags.is_empty()
		&& entry.rationale.decision == "excluded")
}

fn memory_summary_entry_is_invalid_top_of_mind(
	entry: &MemorySummaryEntry,
	non_current_source_refs: &BTreeSet<&str>,
) -> bool {
	entry.category == "top_of_mind"
		&& (entry.freshness.status != "current"
			|| entry.rationale.decision != "included"
			|| !entry.freshness.superseded_by.is_empty()
			|| !entry.freshness.tombstone_refs.is_empty()
			|| entry
				.source_refs
				.iter()
				.any(|source_ref| non_current_source_refs.contains(source_ref.as_str())))
}

fn memory_summary_entry_has_freshness(entry: &MemorySummaryEntry) -> bool {
	if entry.freshness.status.trim().is_empty() {
		return false;
	}

	match entry.category.as_str() {
		"superseded" => !entry.freshness.superseded_by.is_empty(),
		"tombstone" =>
			entry.freshness.status == "tombstoned" && !entry.freshness.tombstone_refs.is_empty(),
		_ => true,
	}
}

fn memory_summary_entry_has_rationale(entry: &MemorySummaryEntry) -> bool {
	!entry.rationale.decision.trim().is_empty()
		&& !entry.rationale.reason_code.trim().is_empty()
		&& !entry.rationale.reason.trim().is_empty()
}

fn memory_summary_entry_includes_unsupported_current_claim(entry: &MemorySummaryEntry) -> bool {
	!entry.unsupported_claim_flags.is_empty()
		&& (entry.rationale.decision != "excluded" || entry.freshness.status == "current")
}
