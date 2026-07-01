use crate::{
	BTreeMap, BTreeSet, JobReport, QuantitativeConfidenceInterval, QuantitativePerQueryRow,
	RealWorldJob, ReportSummary, formatting,
	quantitative::{QUANTITATIVE_K_VALUES, QUANTITATIVE_ROW_CLAIM_BOUNDARY, WILSON_95_Z},
	scoring,
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

pub(super) fn aggregate_metrics(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
	aggregate_metrics_impl(rows)
}

pub(super) fn aggregate_metric_states(
	result_state: &str,
	metric_comparable: bool,
) -> BTreeMap<String, String> {
	aggregate_metric_states_impl(result_state, metric_comparable)
}

pub(super) fn aggregate_denominators(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, usize> {
	aggregate_denominators_impl(rows)
}

pub(super) fn aggregate_confidence_intervals(
	rows: &[QuantitativePerQueryRow],
) -> BTreeMap<String, QuantitativeConfidenceInterval> {
	aggregate_confidence_intervals_impl(rows)
}

pub(super) fn ranking_query_ids(source_jobs: &[RealWorldJob]) -> BTreeSet<&str> {
	ranking_query_ids_impl(source_jobs)
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
	aggregate_qrel_source_impl(ranking_query_count, explicit_qrel_query_count)
}

pub(super) fn ranking_coverage_state(
	summary: &ReportSummary,
	source_job_count: usize,
	ranking_query_count: usize,
) -> &'static str {
	ranking_coverage_state_impl(summary, source_job_count, ranking_query_count)
}

pub(super) fn ranked_candidate_source(ranking_query_count: usize) -> &'static str {
	if ranking_query_count == 0 { "not_encoded" } else { "produced_evidence_order" }
}

fn quantitative_per_query_row(
	source_job: &RealWorldJob,
	job: &JobReport,
	corpus_id: &str,
	evidence_class: &str,
	adapter_id: &str,
) -> QuantitativePerQueryRow {
	let relevance = relevance_grades(source_job, job);
	let candidates = scoring::produced_evidence_order(source_job);
	let positive_relevance_count = positive_qrel_count(&relevance);
	let metrics = per_query_metrics(candidates.as_slice(), &relevance);
	let metric_state = if positive_relevance_count == 0 || candidates.is_empty() {
		"not_encoded"
	} else {
		formatting::status_str(job.status)
	};
	let metric_states = metrics.keys().map(|key| (key.clone(), metric_state.to_string())).collect();
	let denominators = per_query_denominators(candidates.len(), positive_relevance_count);

	QuantitativePerQueryRow {
		job_id: job.job_id.clone(),
		suite: job.suite_id.clone(),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.to_string()),
		result_state: formatting::status_str(job.status).to_string(),
		expected_relevant_count: positive_relevance_count,
		candidate_count: candidates.len(),
		qrel_source: qrel_source(source_job, relevance.is_empty()).to_string(),
		relevance_grade_sum: formatting::round3(relevance.values().sum::<f64>()),
		product: "ELF".to_string(),
		adapter_id: adapter_id.to_string(),
		metrics,
		metric_states,
		denominators,
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	}
}

fn relevance_grades(source_job: &RealWorldJob, job: &JobReport) -> BTreeMap<String, f64> {
	let explicit = source_job
		.expected_answer
		.relevance_judgments
		.iter()
		.map(|judgment| (judgment.evidence_id.clone(), judgment.grade))
		.collect::<BTreeMap<_, _>>();

	if !explicit.is_empty() {
		return explicit;
	}

	job.expected_evidence.iter().map(|evidence| (evidence.evidence_id.clone(), 1.0)).collect()
}

fn per_query_metrics(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> BTreeMap<String, Option<f64>> {
	let mut metrics = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		let relevant_at_k = relevant_at_k(candidates, relevance, *k);

		metrics
			.insert(format!("recall_at_{k}"), rate(relevant_at_k, positive_qrel_count(relevance)));
		metrics.insert(format!("precision_at_{k}"), rate(relevant_at_k, *k));
		metrics.insert(
			format!("success_at_{k}"),
			Some(f64::from(relevant_at_k > 0 && positive_qrel_count(relevance) > 0)),
		);
	}

	metrics.insert("mrr".to_string(), reciprocal_rank(candidates, relevance));
	metrics.insert("ndcg_at_5".to_string(), ndcg_at_k(candidates, relevance, 5));
	metrics.insert("average_precision".to_string(), average_precision(candidates, relevance));

	metrics
}

fn relevant_at_k(candidates: &[String], relevance: &BTreeMap<String, f64>, k: usize) -> usize {
	candidates
		.iter()
		.take(k)
		.filter(|candidate| relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0))
		.count()
}

fn reciprocal_rank(candidates: &[String], relevance: &BTreeMap<String, f64>) -> Option<f64> {
	if positive_qrel_count(relevance) == 0 {
		return None;
	}

	Some(
		candidates
			.iter()
			.position(|candidate| {
				relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0)
			})
			.map_or(0.0, |index| 1.0 / (index + 1) as f64),
	)
}

fn ndcg_at_k(candidates: &[String], relevance: &BTreeMap<String, f64>, k: usize) -> Option<f64> {
	if positive_qrel_count(relevance) == 0 {
		return None;
	}

	let dcg = candidates
		.iter()
		.take(k)
		.enumerate()
		.map(|(index, candidate)| {
			relevance.get(candidate.as_str()).copied().unwrap_or(0.0).max(0.0)
				/ ((index + 2) as f64).log2()
		})
		.sum::<f64>();
	let mut ideal = relevance.values().copied().filter(|grade| *grade > 0.0).collect::<Vec<_>>();

	ideal.sort_by(|left, right| right.total_cmp(left));

	let idcg = ideal
		.iter()
		.take(k)
		.enumerate()
		.map(|(index, grade)| grade / ((index + 2) as f64).log2())
		.sum::<f64>();

	Some(if idcg > 0.0 { dcg / idcg } else { 0.0 })
}

fn average_precision(candidates: &[String], relevance: &BTreeMap<String, f64>) -> Option<f64> {
	let positive_count = positive_qrel_count(relevance);

	if positive_count == 0 {
		return None;
	}

	let mut hit_count = 0;
	let mut precision_sum = 0.0;
	let mut seen = BTreeSet::new();

	for (index, candidate) in candidates.iter().enumerate() {
		if !seen.insert(candidate.as_str()) {
			continue;
		}
		if relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0) {
			hit_count += 1;
			precision_sum += hit_count as f64 / (index + 1) as f64;
		}
	}

	Some(precision_sum / positive_count as f64)
}

fn aggregate_metrics_impl(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
	let mut sums = BTreeMap::<String, (f64, usize)>::new();
	let mut metrics = quantitative_metric_names()
		.into_iter()
		.map(|metric| (metric, None))
		.collect::<BTreeMap<_, _>>();

	for row in rows {
		for (metric, value) in &row.metrics {
			if let Some(value) = value {
				let (sum, count) = sums.entry(metric.clone()).or_default();

				*sum += *value;
				*count += 1;
			}
		}
	}
	for (metric, (sum, count)) in sums {
		metrics.insert(metric, (count > 0).then(|| formatting::round3(sum / count as f64)));
	}

	metrics
}

fn aggregate_metric_states_impl(
	result_state: &str,
	metric_comparable: bool,
) -> BTreeMap<String, String> {
	let state = if metric_comparable { result_state } else { "not_encoded" };
	let mut states = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		states.insert(format!("recall_at_{k}"), state.to_string());
		states.insert(format!("precision_at_{k}"), state.to_string());
		states.insert(format!("success_at_{k}"), state.to_string());
	}
	for metric in ["mrr", "ndcg_at_5", "average_precision"] {
		states.insert(metric.to_string(), state.to_string());
	}

	states
}

fn quantitative_metric_names() -> Vec<String> {
	let mut metrics = Vec::new();

	for k in QUANTITATIVE_K_VALUES {
		metrics.push(format!("recall_at_{k}"));
		metrics.push(format!("precision_at_{k}"));
		metrics.push(format!("success_at_{k}"));
	}
	for metric in ["mrr", "ndcg_at_5", "average_precision"] {
		metrics.push(metric.to_string());
	}

	metrics
}

fn per_query_denominators(
	candidate_count: usize,
	expected_relevant_count: usize,
) -> BTreeMap<String, usize> {
	let mut denominators = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		denominators.insert(format!("recall_at_{k}"), expected_relevant_count);
		denominators.insert(format!("precision_at_{k}"), *k);
		denominators.insert(format!("success_at_{k}"), 1);
	}

	denominators.insert("mrr".to_string(), expected_relevant_count);
	denominators.insert("ndcg_at_5".to_string(), expected_relevant_count.min(5));
	denominators.insert("average_precision".to_string(), expected_relevant_count);
	denominators.insert("candidate_count".to_string(), candidate_count);

	denominators
}

fn aggregate_denominators_impl(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, usize> {
	let mut denominators = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		denominators.insert(
			format!("recall_at_{k}"),
			sum_per_query_denominator(rows, &format!("recall_at_{k}")),
		);
		denominators.insert(
			format!("precision_at_{k}"),
			sum_per_query_denominator(rows, &format!("precision_at_{k}")),
		);
		denominators.insert(
			format!("success_at_{k}"),
			sum_per_query_denominator(rows, &format!("success_at_{k}")),
		);
	}

	denominators.insert("mrr".to_string(), sum_per_query_denominator(rows, "mrr"));
	denominators.insert("ndcg_at_5".to_string(), sum_per_query_denominator(rows, "ndcg_at_5"));
	denominators.insert(
		"average_precision".to_string(),
		sum_per_query_denominator(rows, "average_precision"),
	);

	denominators
}

fn aggregate_confidence_intervals_impl(
	rows: &[QuantitativePerQueryRow],
) -> BTreeMap<String, QuantitativeConfidenceInterval> {
	let mut confidence_intervals = BTreeMap::new();

	for metric in rate_metric_names() {
		let (numerator, denominator) = aggregate_rate_numerator_denominator(rows, metric.as_str());

		if denominator > 0 {
			confidence_intervals.insert(
				metric,
				wilson_confidence_interval(numerator.min(denominator), denominator),
			);
		}
	}

	confidence_intervals
}

fn rate_metric_names() -> Vec<String> {
	let mut metrics = Vec::new();

	for k in QUANTITATIVE_K_VALUES {
		metrics.push(format!("recall_at_{k}"));
		metrics.push(format!("precision_at_{k}"));
		metrics.push(format!("success_at_{k}"));
	}

	metrics
}

fn aggregate_rate_numerator_denominator(
	rows: &[QuantitativePerQueryRow],
	metric: &str,
) -> (usize, usize) {
	let mut numerator = 0;
	let mut denominator = 0;

	for row in rows {
		let Some(value) = row.metrics.get(metric).and_then(|value| *value) else {
			continue;
		};
		let Some(row_denominator) = row.denominators.get(metric).copied() else {
			continue;
		};

		if row_denominator == 0 {
			continue;
		}

		denominator += row_denominator;
		numerator += (value * row_denominator as f64).round() as usize;
	}

	(numerator, denominator)
}

fn wilson_confidence_interval(
	numerator: usize,
	denominator: usize,
) -> QuantitativeConfidenceInterval {
	let n = denominator as f64;
	let p = numerator as f64 / n;
	let z2 = WILSON_95_Z * WILSON_95_Z;
	let center = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
	let half_width =
		WILSON_95_Z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / (1.0 + z2 / n);

	QuantitativeConfidenceInterval {
		method: "wilson_score".to_string(),
		confidence: 0.95,
		lower: formatting::round3((center - half_width).clamp(0.0, 1.0)),
		upper: formatting::round3((center + half_width).clamp(0.0, 1.0)),
		numerator,
		denominator,
	}
}

fn sum_per_query_denominator(rows: &[QuantitativePerQueryRow], metric: &str) -> usize {
	rows.iter().filter_map(|row| row.denominators.get(metric)).sum()
}

fn ranking_query_ids_impl(source_jobs: &[RealWorldJob]) -> BTreeSet<&str> {
	source_jobs
		.iter()
		.filter(|job| !ranking_relevance_grades(job).is_empty() && ranking_query_attempted(job))
		.map(|job| job.job_id.as_str())
		.collect()
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

fn qrel_source(source_job: &RealWorldJob, empty: bool) -> &'static str {
	if !source_job.expected_answer.relevance_judgments.is_empty() {
		"explicit_qrels"
	} else if empty {
		"not_encoded"
	} else {
		"expected_evidence_fallback"
	}
}

fn aggregate_qrel_source_impl(
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

fn ranking_coverage_state_impl(
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

fn positive_qrel_count(relevance: &BTreeMap<String, f64>) -> usize {
	relevance.values().filter(|grade| **grade > 0.0).count()
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
	(denominator > 0).then(|| formatting::round3(numerator as f64 / denominator as f64))
}
