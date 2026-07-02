use crate::ReportSummary;

pub(in crate::quantitative) fn ranking_coverage_state(
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

pub(in crate::quantitative) fn ranked_candidate_source(ranking_query_count: usize) -> &'static str {
	if ranking_query_count == 0 { "not_encoded" } else { "produced_evidence_order" }
}
