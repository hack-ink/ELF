use crate::{
	BTreeMap, JobReport, RealWorldJob, formatting,
	quantitative::metrics::per_query::{evidence, query_metrics},
	scoring,
};

pub(super) struct QuantitativePerQueryRowBasis {
	pub(super) positive_relevance_count: usize,
	pub(super) candidate_count: usize,
	pub(super) qrel_source: String,
	pub(super) relevance_grade_sum: f64,
	pub(super) metrics: BTreeMap<String, Option<f64>>,
	pub(super) metric_states: BTreeMap<String, String>,
	pub(super) denominators: BTreeMap<String, usize>,
}

pub(super) fn quantitative_per_query_row_basis(
	source_job: &RealWorldJob,
	job: &JobReport,
) -> QuantitativePerQueryRowBasis {
	let relevance = evidence::relevance_grades(source_job, job);
	let candidates = scoring::produced_evidence_order(source_job);
	let positive_relevance_count = query_metrics::positive_qrel_count(&relevance);
	let metrics = query_metrics::per_query_metrics(candidates.as_slice(), &relevance);
	let candidate_count = candidates.len();
	let metric_states = per_query_metric_states(
		metrics.keys(),
		positive_relevance_count,
		candidate_count,
		formatting::status_str(job.status),
	);

	QuantitativePerQueryRowBasis {
		positive_relevance_count,
		candidate_count,
		qrel_source: evidence::qrel_source(source_job, relevance.is_empty()).to_string(),
		relevance_grade_sum: formatting::round3(relevance.values().sum::<f64>()),
		metrics,
		metric_states,
		denominators: query_metrics::per_query_denominators(
			candidate_count,
			positive_relevance_count,
		),
	}
}

fn per_query_metric_states<'a>(
	metric_names: impl Iterator<Item = &'a String>,
	positive_relevance_count: usize,
	candidate_count: usize,
	result_state: &str,
) -> BTreeMap<String, String> {
	let metric_state = if positive_relevance_count == 0 || candidate_count == 0 {
		"not_encoded"
	} else {
		result_state
	};

	metric_names.map(|key| (key.clone(), metric_state.to_string())).collect()
}
