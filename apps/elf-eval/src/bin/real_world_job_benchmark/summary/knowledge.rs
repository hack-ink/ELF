use super::{super::formatting::round3, *};

pub(super) fn knowledge_summary_impl(jobs: &[JobReport]) -> Option<KnowledgeSummary> {
	let knowledge_jobs = jobs.iter().filter_map(|job| job.knowledge.as_ref()).collect::<Vec<_>>();

	if knowledge_jobs.is_empty() {
		return None;
	}

	let job_count = knowledge_jobs.len();
	let page_count = knowledge_jobs.iter().map(|metrics| metrics.page_count).sum::<usize>();
	let section_count = knowledge_jobs.iter().map(|metrics| metrics.section_count).sum::<usize>();
	let traced_section_count =
		knowledge_jobs.iter().map(|metrics| metrics.traced_section_count).sum::<usize>();
	let stale_trap_count =
		knowledge_jobs.iter().map(|metrics| metrics.stale_trap_count).sum::<usize>();
	let stale_traps_detected =
		knowledge_jobs.iter().map(|metrics| metrics.stale_traps_detected).sum::<usize>();
	let deterministic_rebuild_count =
		knowledge_jobs.iter().map(|metrics| metrics.deterministic_rebuild_count).sum::<usize>();
	let rebuild_page_count =
		knowledge_jobs.iter().map(|metrics| metrics.rebuild_page_count).sum::<usize>();
	let backlink_count = knowledge_jobs.iter().map(|metrics| metrics.backlink_count).sum::<usize>();
	let pages_with_backlinks =
		knowledge_jobs.iter().map(|metrics| metrics.pages_with_backlinks).sum::<usize>();
	let pages_with_version_diff =
		knowledge_jobs.iter().map(|metrics| metrics.pages_with_version_diff).sum::<usize>();
	let page_usefulness = round3(
		knowledge_jobs.iter().map(|metrics| metrics.page_usefulness).sum::<f64>()
			/ job_count as f64,
	);

	Some(KnowledgeSummary {
		job_count,
		page_count,
		section_count,
		backlink_count,
		pages_with_backlinks,
		pages_with_version_diff,
		citation_coverage: ratio(traced_section_count, section_count),
		stale_claim_detection: ratio_or_full(stale_traps_detected, stale_trap_count),
		rebuild_determinism: ratio(deterministic_rebuild_count, rebuild_page_count),
		backlink_coverage: ratio(pages_with_backlinks, page_count),
		version_diff_coverage: ratio(pages_with_version_diff, page_count),
		page_usefulness,
		unsupported_summary_count: knowledge_jobs
			.iter()
			.map(|metrics| metrics.unsupported_summary_count)
			.sum(),
		untraced_section_count: knowledge_jobs
			.iter()
			.map(|metrics| metrics.untraced_section_count)
			.sum(),
		allowed_variance_count: knowledge_jobs
			.iter()
			.map(|metrics| metrics.allowed_variance_count)
			.sum(),
	})
}
