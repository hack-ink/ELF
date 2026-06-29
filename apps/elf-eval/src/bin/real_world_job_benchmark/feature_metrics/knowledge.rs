use crate::feature_metrics::{
	self, DerivedPageArtifact, DerivedPageRebuild, DerivedPageSection, KnowledgeJobMetrics,
	NegativeTrap, ProducedAnswer, RealWorldJob, UnsupportedClaimReport, Value,
};

pub(super) fn unsupported_page_claims_impl(answer: &ProducedAnswer) -> Vec<UnsupportedClaimReport> {
	answer
		.pages
		.iter()
		.flat_map(|page| {
			page.sections.iter().filter_map(|section| {
				if section_is_traced(section) || section_is_flagged_unsupported(section) {
					return None;
				}

				Some(UnsupportedClaimReport {
					suite_id: String::new(),
					job_id: String::new(),
					claim_id: Some(format!("{}:{}", page.page_id, section.section_id)),
					claim_text: feature_metrics::bounded_text(section.content.as_str(), 240),
					reason:
						"derived page section has no source evidence and is not flagged unsupported"
							.to_string(),
					evidence_ids: section.evidence_ids.clone(),
				})
			})
		})
		.collect()
}

pub(super) fn knowledge_metrics_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<KnowledgeJobMetrics> {
	if answer.pages.is_empty() {
		return None;
	}

	let mut metrics = KnowledgeJobMetrics {
		page_count: answer.pages.len(),
		stale_trap_count: stale_traps(job).len(),
		..KnowledgeJobMetrics::default()
	};

	for page in &answer.pages {
		accumulate_page_metrics(page, &mut metrics);
	}

	metrics.stale_traps_detected = stale_traps(job)
		.iter()
		.filter(|trap| page_artifacts_detect_stale_trap(&answer.pages, trap))
		.count();
	metrics.citation_coverage =
		feature_metrics::ratio(metrics.traced_section_count, metrics.section_count);
	metrics.stale_claim_detection =
		feature_metrics::ratio_or_full(metrics.stale_traps_detected, metrics.stale_trap_count);
	metrics.rebuild_determinism =
		feature_metrics::ratio(metrics.deterministic_rebuild_count, metrics.page_count);
	metrics.backlink_coverage =
		feature_metrics::ratio(metrics.pages_with_backlinks, metrics.page_count);
	metrics.version_diff_coverage =
		feature_metrics::ratio(metrics.pages_with_version_diff, metrics.page_count);
	metrics.page_usefulness = feature_metrics::round3(
		(metrics.citation_coverage
			+ metrics.stale_claim_detection
			+ metrics.rebuild_determinism
			+ metrics.backlink_coverage)
			/ 4.0,
	);

	Some(metrics)
}

pub(super) fn missed_stale_finding_count_impl(metrics: &KnowledgeJobMetrics) -> usize {
	metrics.stale_trap_count.saturating_sub(metrics.stale_traps_detected)
}

pub(super) fn page_usefulness_failure_count_impl(metrics: &KnowledgeJobMetrics) -> usize {
	if metrics.page_usefulness < 0.8 { 1 } else { 0 }
}

fn stale_traps(job: &RealWorldJob) -> Vec<&NegativeTrap> {
	job.negative_traps
		.iter()
		.filter(|trap| trap.trap_type == "stale_fact" && trap.failure_if_used)
		.collect()
}

fn accumulate_page_metrics(page: &DerivedPageArtifact, metrics: &mut KnowledgeJobMetrics) {
	if !page.backlinks.is_empty() {
		metrics.pages_with_backlinks += 1;
	}
	if page_has_version_diff(page) {
		metrics.pages_with_version_diff += 1;
	}

	metrics.backlink_count += page.backlinks.len();

	for section in &page.sections {
		metrics.section_count += 1;

		if section_is_traced(section) {
			metrics.traced_section_count += 1;
		} else if section_is_flagged_unsupported(section) {
			metrics.flagged_unsupported_section_count += 1;

			if section.role == "summary" {
				metrics.unsupported_summary_count += 1;
			}
		} else {
			metrics.untraced_section_count += 1;
		}
	}

	if let Some(rebuild) = &page.rebuild {
		if !rebuild.allowed_variance.is_empty() {
			metrics.allowed_variance_count += 1;
		}
		if rebuild_is_acceptable(rebuild) {
			metrics.deterministic_rebuild_count += 1;
		} else {
			metrics.rebuild_failure_count += 1;
		}
	} else {
		metrics.rebuild_failure_count += 1;
	}

	metrics.rebuild_page_count += 1;
}

fn page_has_version_diff(page: &DerivedPageArtifact) -> bool {
	page.page_version_diff.as_ref().is_some_and(|diff| {
		diff.get("schema").and_then(Value::as_str) == Some("elf.knowledge_page.version_diff/v1")
			&& diff.get("available").and_then(Value::as_bool).unwrap_or(false)
	})
}

fn section_is_traced(section: &DerivedPageSection) -> bool {
	!section.evidence_ids.is_empty() || !section.timeline_event_ids.is_empty()
}

fn section_is_flagged_unsupported(section: &DerivedPageSection) -> bool {
	section.unsupported_reason.as_ref().is_some_and(|reason| !reason.trim().is_empty())
}

fn rebuild_is_acceptable(rebuild: &DerivedPageRebuild) -> bool {
	(rebuild.deterministic && rebuild.first_hash == rebuild.second_hash)
		|| !rebuild.allowed_variance.is_empty()
}

fn page_artifacts_detect_stale_trap(pages: &[DerivedPageArtifact], trap: &NegativeTrap) -> bool {
	pages.iter().any(|page| {
		page.lint_findings.iter().any(|finding| {
			finding.trap_id.as_deref() == Some(trap.trap_id.as_str())
				|| finding
					.evidence_ids
					.iter()
					.any(|evidence_id| trap.evidence_ids.contains(evidence_id))
		})
	})
}
