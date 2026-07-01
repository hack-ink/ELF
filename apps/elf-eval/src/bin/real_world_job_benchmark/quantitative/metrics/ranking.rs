use crate::{BTreeMap, BTreeSet, RealWorldJob, ReportSummary, scoring};

pub(super) fn ranking_query_ids(source_jobs: &[RealWorldJob]) -> BTreeSet<&str> {
	source_jobs
		.iter()
		.filter(|job| !ranking_relevance_grades(job).is_empty() && ranking_query_attempted(job))
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

fn ranking_relevance_grades(source_job: &RealWorldJob) -> BTreeMap<String, f64> {
	if !source_job.expected_answer.relevance_judgments.is_empty() {
		return source_job
			.expected_answer
			.relevance_judgments
			.iter()
			.filter(|judgment| judgment.grade > 0.0)
			.map(|judgment| (judgment.evidence_id.clone(), judgment.grade))
			.collect();
	}

	source_job
		.required_evidence
		.iter()
		.filter(|evidence| matches!(evidence.requirement.as_str(), "cite" | "use" | "explain"))
		.map(|evidence| (evidence.evidence_id.clone(), 1.0))
		.collect()
}

fn ranking_query_attempted(job: &RealWorldJob) -> bool {
	if !scoring::produced_evidence_order(job).is_empty() {
		return true;
	}

	let Some(answer) = job.corpus.adapter_response.as_ref().map(|response| &response.answer) else {
		return false;
	};

	answer.trace_explainability.as_ref().is_some_and(|trace| {
		trace.stages.iter().any(|stage| stage.stage_name == "live_adapter.retrieve")
	}) && answer.latency_ms.is_some_and(|latency| latency.is_finite() && latency > 0.0)
}
