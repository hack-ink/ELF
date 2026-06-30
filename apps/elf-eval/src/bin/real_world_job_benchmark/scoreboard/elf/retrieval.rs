use crate::scoreboard::{
	self, BTreeSet, JobReport, ReportSummary, SCOREBOARD_RETRIEVAL_K, ScoreboardRankedMetrics,
	ScoreboardRetrievalMetrics, common,
};

pub(in crate::scoreboard::elf) fn scoreboard_retrieval_metrics(
	job_reports: &[JobReport],
	summary: &ReportSummary,
) -> ScoreboardRetrievalMetrics {
	let produced_evidence_total =
		job_reports.iter().map(|job| job.retrieval_quality.produced_evidence_total).sum();
	let mut relevant_at_k = 0;
	let mut precision_denominator_at_k = 0;
	let mut reciprocal_rank_sum = 0.0;
	let mut ndcg_sum = 0.0;
	let mut ranked_job_count = 0;

	for job in job_reports {
		let expected = job
			.expected_evidence
			.iter()
			.map(|evidence| evidence.evidence_id.as_str())
			.collect::<BTreeSet<_>>();
		let ranked = scoreboard_ranked_metrics_for_job(job, &expected);

		relevant_at_k += ranked.relevant_at_k;
		precision_denominator_at_k += ranked.precision_denominator_at_k;
		reciprocal_rank_sum += ranked.reciprocal_rank;
		ndcg_sum += ranked.ndcg;
		ranked_job_count += 1;
	}

	ScoreboardRetrievalMetrics {
		k: SCOREBOARD_RETRIEVAL_K,
		metric_basis: "produced_evidence_order".to_string(),
		recall_at_k: Some(scoreboard::ratio_or(
			relevant_at_k,
			summary.expected_evidence_total,
			1.0,
		)),
		precision_at_k: Some(scoreboard::ratio_or(relevant_at_k, precision_denominator_at_k, 1.0)),
		mrr: Some(common::scoreboard_mean_metric(reciprocal_rank_sum, ranked_job_count)),
		ndcg: Some(common::scoreboard_mean_metric(ndcg_sum, ranked_job_count)),
		expected_evidence_recall: Some(summary.expected_evidence_recall),
		citation_source_ref_coverage: Some(summary.source_ref_coverage),
		expected_evidence_matched: summary.expected_evidence_matched,
		expected_evidence_total: summary.expected_evidence_total,
		produced_evidence_total,
	}
}

fn scoreboard_ranked_metrics_for_job(
	job: &JobReport,
	expected: &BTreeSet<&str>,
) -> ScoreboardRankedMetrics {
	let precision_denominator_at_k = SCOREBOARD_RETRIEVAL_K;
	let relevant_at_k = job
		.produced_evidence
		.iter()
		.take(SCOREBOARD_RETRIEVAL_K)
		.filter(|evidence_id| expected.contains(evidence_id.as_str()))
		.count();
	let reciprocal_rank = job
		.produced_evidence
		.iter()
		.position(|evidence_id| expected.contains(evidence_id.as_str()))
		.map_or_else(|| f64::from(expected.is_empty()), |index| 1.0 / (index + 1) as f64);
	let ndcg = scoreboard_ndcg(job.produced_evidence.as_slice(), expected);

	ScoreboardRankedMetrics { relevant_at_k, precision_denominator_at_k, reciprocal_rank, ndcg }
}

fn scoreboard_ndcg(produced_evidence: &[String], expected: &BTreeSet<&str>) -> f64 {
	if expected.is_empty() {
		return 1.0;
	}

	let dcg = produced_evidence
		.iter()
		.take(SCOREBOARD_RETRIEVAL_K)
		.enumerate()
		.filter(|(_, evidence_id)| expected.contains(evidence_id.as_str()))
		.map(|(index, _)| 1.0 / ((index + 2) as f64).log2())
		.sum::<f64>();
	let ideal_hits = expected.len().min(SCOREBOARD_RETRIEVAL_K);
	let idcg = (0..ideal_hits).map(|index| 1.0 / ((index + 2) as f64).log2()).sum::<f64>();

	if idcg > 0.0 { dcg / idcg } else { 0.0 }
}
