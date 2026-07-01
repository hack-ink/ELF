use crate::{
	BTreeMap, BTreeSet, JobReport, QuantitativePerQueryRow, RealWorldJob, formatting,
	quantitative::{QUANTITATIVE_K_VALUES, QUANTITATIVE_ROW_CLAIM_BOUNDARY},
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

fn qrel_source(source_job: &RealWorldJob, empty: bool) -> &'static str {
	if !source_job.expected_answer.relevance_judgments.is_empty() {
		"explicit_qrels"
	} else if empty {
		"not_encoded"
	} else {
		"expected_evidence_fallback"
	}
}

fn positive_qrel_count(relevance: &BTreeMap<String, f64>) -> usize {
	relevance.values().filter(|grade| **grade > 0.0).count()
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
	(denominator > 0).then(|| formatting::round3(numerator as f64 / denominator as f64))
}
