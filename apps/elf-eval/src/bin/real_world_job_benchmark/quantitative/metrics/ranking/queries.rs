use crate::{BTreeMap, RealWorldJob, scoring};

pub(super) fn is_ranking_query(job: &RealWorldJob) -> bool {
	!ranking_relevance_grades(job).is_empty() && ranking_query_attempted(job)
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
