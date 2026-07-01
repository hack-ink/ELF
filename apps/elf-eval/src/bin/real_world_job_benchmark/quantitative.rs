use crate::{
	AdapterReport, BTreeMap, BTreeSet, JobReport, QuantitativeBenchmarkControls,
	QuantitativeBenchmarkReport, QuantitativeBenchmarkRow, QuantitativePerQueryRow, RealWorldJob,
	ReportSummary, formatting, scoring,
};

const QUANTITATIVE_SCOREBOARD_SCHEMA: &str = "elf.agent_memory_quantitative_benchmark/v1";
const QUANTITATIVE_K_VALUES: &[usize] = &[1, 3, 5, 10];
const MIN_LEADERBOARD_QUERY_COUNT: usize = 30;
const QUANTITATIVE_ROW_CLAIM_BOUNDARY: &str = concat!(
	"Quantitative metrics are bounded to this generated report. ",
	"Fixture-backed rows prove benchmark mechanics, not product-runtime or leaderboard claims."
);

pub(super) struct QuantitativeReportInput<'a> {
	pub(super) generated_at: &'a str,
	pub(super) adapter: &'a AdapterReport,
	pub(super) source_jobs: &'a [RealWorldJob],
	pub(super) jobs: &'a [JobReport],
	pub(super) summary: &'a ReportSummary,
}

pub(super) fn quantitative_scoreboard_report(
	input: QuantitativeReportInput<'_>,
) -> QuantitativeBenchmarkReport {
	let corpus_id = quantitative_corpus_id(input.source_jobs);
	let evidence_class = quantitative_evidence_class(input.adapter, input.jobs);
	let per_query_rows = quantitative_per_query_rows(
		input.source_jobs,
		input.jobs,
		corpus_id.as_str(),
		evidence_class,
		input.adapter.adapter_id.as_str(),
	);
	let ranking_query_count = per_query_rows
		.iter()
		.filter(|row| row.candidate_count > 0 && row.expected_relevant_count > 0)
		.count();
	let explicit_qrel_query_count =
		per_query_rows.iter().filter(|row| row.qrel_source == "explicit_qrels").count();
	let metric_comparable = ranking_query_count > 0;
	let leaderboard_eligible = false;
	let result_state = quantitative_result_state(input.summary);
	let row = QuantitativeBenchmarkRow {
		product: "ELF".to_string(),
		adapter_id: input.adapter.adapter_id.clone(),
		adapter_name: input.adapter.name.clone(),
		suite: quantitative_suite_id(input.jobs),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.clone()),
		result_state: result_state.to_string(),
		comparable: metric_comparable,
		metric_comparable,
		leaderboard_eligible,
		held_out: false,
		leakage_audited: false,
		fixture_regression_only: evidence_class == "fixture_backed",
		sample_size: input.jobs.len(),
		ranking_query_count,
		ranking_coverage_state: ranking_coverage_state(
			input.summary,
			input.source_jobs.len(),
			ranking_query_count,
		)
		.to_string(),
		ranked_candidate_source: ranked_candidate_source(ranking_query_count).to_string(),
		qrel_source: aggregate_qrel_source(ranking_query_count, explicit_qrel_query_count)
			.to_string(),
		explicit_qrel_query_count,
		metrics: aggregate_metrics(per_query_rows.as_slice()),
		metric_states: aggregate_metric_states(result_state, metric_comparable),
		denominators: aggregate_denominators(per_query_rows.as_slice()),
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	};
	let controls = QuantitativeBenchmarkControls {
		same_corpus_required: true,
		same_task_required: true,
		ranked_candidates_required_for_ranking_metrics: true,
		explicit_relevance_judgments_required_for_leaderboard: true,
		minimum_query_count_for_leaderboard: MIN_LEADERBOARD_QUERY_COUNT,
		current_query_count: input.source_jobs.len(),
		current_ranking_query_count: ranking_query_count,
		current_explicit_qrel_query_count: explicit_qrel_query_count,
		leaderboard_claim_allowed: leaderboard_eligible,
		leakage_control:
			"held_out_or_leakage_audited_runtime_rows_required_before_leaderboard_claims"
				.to_string(),
	};

	QuantitativeBenchmarkReport {
		schema: QUANTITATIVE_SCOREBOARD_SCHEMA.to_string(),
		generated_at: input.generated_at.to_string(),
		corpus_id,
		k_values: QUANTITATIVE_K_VALUES.to_vec(),
		rows: vec![row],
		per_query_rows,
		metrics_not_encoded: vec![
			"paired_significance".to_string(),
			"external_product_manifest_import".to_string(),
			"audit_manifest_validation".to_string(),
		],
		controls,
		claim_boundary: concat!(
			"Do not convert fixture mechanics, missing explicit qrels, ",
			"or partial candidate coverage into product leaderboard claims."
		)
		.to_string(),
	}
}

fn quantitative_per_query_rows(
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

fn aggregate_metrics(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
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

fn aggregate_metric_states(
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

fn aggregate_denominators(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, usize> {
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

fn sum_per_query_denominator(rows: &[QuantitativePerQueryRow], metric: &str) -> usize {
	rows.iter().filter_map(|row| row.denominators.get(metric)).sum()
}

fn quantitative_corpus_id(source_jobs: &[RealWorldJob]) -> String {
	let ids = source_jobs.iter().map(|job| job.corpus.corpus_id.as_str()).collect::<BTreeSet<_>>();

	if ids.len() == 1 {
		ids.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn quantitative_suite_id(jobs: &[JobReport]) -> String {
	let suites = jobs.iter().map(|job| job.suite_id.as_str()).collect::<BTreeSet<_>>();

	if suites.len() == 1 {
		suites.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn quantitative_result_state(summary: &ReportSummary) -> &'static str {
	if summary.unsupported_claim > 0 {
		"unsupported_claim"
	} else if summary.wrong_result > 0 {
		"wrong_result"
	} else if summary.incomplete > 0 {
		"incomplete"
	} else if summary.blocked > 0 {
		"blocked"
	} else if summary.not_encoded > 0 {
		"not_encoded"
	} else {
		"pass"
	}
}

fn quantitative_evidence_class(adapter: &AdapterReport, jobs: &[JobReport]) -> &'static str {
	if adapter.behavior == "live_real_world_adapter" {
		"live_real_world"
	} else if jobs.iter().any(|job| job.operational_evidence_tier == "private_corpus") {
		"private_corpus"
	} else if jobs.iter().any(|job| job.operational_evidence_tier == "provider_backed") {
		"provider_backed"
	} else if adapter.behavior.contains("public_proxy") {
		"public_proxy"
	} else {
		"fixture_backed"
	}
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

fn aggregate_qrel_source(
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

fn ranking_coverage_state(
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

fn ranked_candidate_source(ranking_query_count: usize) -> &'static str {
	if ranking_query_count == 0 { "not_encoded" } else { "produced_evidence_order" }
}

fn positive_qrel_count(relevance: &BTreeMap<String, f64>) -> usize {
	relevance.values().filter(|grade| **grade > 0.0).count()
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
	(denominator > 0).then(|| formatting::round3(numerator as f64 / denominator as f64))
}
