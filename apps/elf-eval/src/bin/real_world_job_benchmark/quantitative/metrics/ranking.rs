mod queries;

use crate::{BTreeSet, RealWorldJob, ReportSummary};

pub(super) fn ranking_query_ids(source_jobs: &[RealWorldJob]) -> BTreeSet<&str> {
	source_jobs
		.iter()
		.filter(|job| queries::is_ranking_query(job))
		.map(|job| job.job_id.as_str())
		.collect()
}

pub(super) fn ranking_query_count(source_jobs: &[RealWorldJob]) -> usize {
	ranking_query_ids(source_jobs).len()
}

pub(super) fn explicit_qrel_query_count(source_jobs: &[RealWorldJob]) -> usize {
	source_jobs.iter().filter(|job| !job.expected_answer.relevance_judgments.is_empty()).count()
}

pub(super) fn aggregate_qrel_source(
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
) -> &'static str {
	if ranking_query_count == 0 {
		"not_encoded"
	} else if explicit_qrel_query_count == ranking_query_count {
		"explicit_qrels"
	} else if explicit_qrel_query_count == 0 {
		"expected_evidence_fallback"
	} else {
		"mixed"
	}
}

pub(super) fn ranking_coverage_state(
	summary: &ReportSummary,
	source_job_count: usize,
	ranking_query_count: usize,
) -> &'static str {
	if ranking_query_count == 0 {
		"not_encoded"
	} else if ranking_query_count == source_job_count && summary.not_encoded == 0 {
		"complete"
	} else {
		"partial_coverage"
	}
}

pub(super) fn ranked_candidate_source(ranking_query_count: usize) -> &'static str {
	if ranking_query_count == 0 { "not_encoded" } else { "produced_evidence_order" }
}
