mod basis;

use crate::{
	JobReport, QuantitativePerQueryRow, RealWorldJob, formatting,
	quantitative::QUANTITATIVE_ROW_CLAIM_BOUNDARY,
};

pub(super) fn quantitative_per_query_row(
	source_job: &RealWorldJob,
	job: &JobReport,
	corpus_id: &str,
	evidence_class: &str,
	adapter_id: &str,
) -> QuantitativePerQueryRow {
	let basis = basis::quantitative_per_query_row_basis(source_job, job);

	QuantitativePerQueryRow {
		job_id: job.job_id.clone(),
		suite: job.suite_id.clone(),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.to_string()),
		result_state: formatting::status_str(job.status).to_string(),
		expected_relevant_count: basis.positive_relevance_count,
		candidate_count: basis.candidate_count,
		qrel_source: basis.qrel_source,
		relevance_grade_sum: basis.relevance_grade_sum,
		product: "ELF".to_string(),
		adapter_id: adapter_id.to_string(),
		metrics: basis.metrics,
		metric_states: basis.metric_states,
		denominators: basis.denominators,
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	}
}
