mod evidence;
mod query_metrics;

use crate::{
	JobReport, QuantitativePerQueryRow, RealWorldJob, formatting,
	quantitative::QUANTITATIVE_ROW_CLAIM_BOUNDARY, scoring,
};

pub(super) fn quantitative_per_query_rows(
	source_jobs: &[RealWorldJob],
	jobs: &[JobReport],
	corpus_id: &str,
	evidence_class: &str,
	adapter_id: &str,
) -> Vec<QuantitativePerQueryRow> {
	source_jobs
		.iter()
		.zip(jobs.iter())
		.map(|(source_job, job)| {
			quantitative_per_query_row(source_job, job, corpus_id, evidence_class, adapter_id)
		})
		.collect()
}

fn quantitative_per_query_row(
	source_job: &RealWorldJob,
	job: &JobReport,
	corpus_id: &str,
	evidence_class: &str,
	adapter_id: &str,
) -> QuantitativePerQueryRow {
	let relevance = evidence::relevance_grades(source_job, job);
	let candidates = scoring::produced_evidence_order(source_job);
	let positive_relevance_count = query_metrics::positive_qrel_count(&relevance);
	let metrics = query_metrics::per_query_metrics(candidates.as_slice(), &relevance);
	let metric_state = if positive_relevance_count == 0 || candidates.is_empty() {
		"not_encoded"
	} else {
		formatting::status_str(job.status)
	};
	let metric_states = metrics.keys().map(|key| (key.clone(), metric_state.to_string())).collect();

	QuantitativePerQueryRow {
		job_id: job.job_id.clone(),
		suite: job.suite_id.clone(),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.to_string()),
		result_state: formatting::status_str(job.status).to_string(),
		expected_relevant_count: positive_relevance_count,
		candidate_count: candidates.len(),
		qrel_source: evidence::qrel_source(source_job, relevance.is_empty()).to_string(),
		relevance_grade_sum: formatting::round3(relevance.values().sum::<f64>()),
		product: "ELF".to_string(),
		adapter_id: adapter_id.to_string(),
		metrics,
		metric_states,
		denominators: query_metrics::per_query_denominators(
			candidates.len(),
			positive_relevance_count,
		),
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	}
}
