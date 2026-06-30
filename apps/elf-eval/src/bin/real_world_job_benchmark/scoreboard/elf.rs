use crate::scoreboard::{
	self, BTreeSet, JobReport, RealWorldJob, ReportSummary, SCOREBOARD_RETRIEVAL_K,
	ScoreboardAnswerSafetyMetrics, ScoreboardCoverageMetrics, ScoreboardLifecycleMetrics,
	ScoreboardMetrics, ScoreboardOperationalMetrics, ScoreboardRankedMetrics,
	ScoreboardRetrievalMetrics, ScoreboardRow, TypedStatus, common,
};

pub(super) fn elf_scoreboard_row(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
) -> ScoreboardRow {
	let source_id_mapped =
		summary.source_ref_required_count > 0 && summary.source_ref_coverage >= 1.0;
	let result_state = common::aggregate_job_report_state(job_reports);
	let metrics = scoreboard_metrics_for_reports(raw_jobs, job_reports, summary);
	let typed_non_pass_count =
		job_reports.iter().filter(|job| job.status != TypedStatus::Pass).count();
	let mut row = ScoreboardRow {
		product_id: "elf_current_report".to_string(),
		product_name: "ELF".to_string(),
		row_source: "current_real_world_job_report".to_string(),
		evidence_class: "fixture_backed".to_string(),
		result_state,
		comparable: false,
		same_corpus: true,
		source_id_mapped,
		held_out: common::jobs_have_tag(raw_jobs, "held_out"),
		leakage_audited: common::jobs_have_tag(raw_jobs, "leakage_audited"),
		product_runtime: false,
		container_digest_identified: false,
		metrics,
		strengths: elf_scoreboard_strengths(summary),
		weaknesses: Vec::new(),
		next_evidence: Vec::new(),
		source_provenance: vec![
			"apps/elf-eval/fixtures/real_world_memory/".to_string(),
			"apps/elf-eval/src/bin/real_world_job_benchmark/main.rs".to_string(),
		],
	};

	if typed_non_pass_count > 0 {
		row.weaknesses
			.push(format!("{typed_non_pass_count} encoded job row(s) are typed non-pass."));
	}

	common::scoreboard_apply_comparability_gaps(&mut row);

	row
}

fn scoreboard_metrics_for_reports(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
) -> ScoreboardMetrics {
	ScoreboardMetrics {
		retrieval: scoreboard_retrieval_metrics(job_reports, summary),
		lifecycle: scoreboard_lifecycle_metrics(raw_jobs, job_reports),
		answer_safety: scoreboard_answer_safety_metrics(summary),
		operations: scoreboard_operational_metrics(raw_jobs, job_reports, summary),
		coverage: ScoreboardCoverageMetrics {
			job_count: summary.job_count,
			encoded_suite_count: summary.encoded_suite_count,
			pass_count: summary.pass,
			typed_non_pass_count: job_reports
				.iter()
				.filter(|job| job.status != TypedStatus::Pass)
				.count(),
			source_ref_coverage: Some(summary.source_ref_coverage),
			evidence_coverage: Some(summary.evidence_coverage),
			evidence_class: "fixture_backed".to_string(),
		},
	}
}

fn scoreboard_retrieval_metrics(
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

fn scoreboard_lifecycle_metrics(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
) -> ScoreboardLifecycleMetrics {
	let stale_check_count: usize = raw_jobs
		.iter()
		.map(|job| {
			job.negative_traps
				.iter()
				.filter(|trap| trap.failure_if_used && trap.trap_type == "stale_fact")
				.count()
		})
		.sum();
	let stale_failure_count = job_reports
		.iter()
		.map(|job| job.stale_answer_count + job.stale_retrieval_count)
		.sum::<usize>();
	let update_check_count =
		scoreboard_lifecycle_check_count(raw_jobs, common::scoreboard_is_update_job);
	let update_correct_count =
		scoreboard_lifecycle_correct_count(raw_jobs, job_reports, common::scoreboard_is_update_job);
	let delete_check_count =
		scoreboard_lifecycle_check_count(raw_jobs, common::scoreboard_is_delete_job);
	let delete_correct_count =
		scoreboard_lifecycle_correct_count(raw_jobs, job_reports, common::scoreboard_is_delete_job);
	let rollback_history_check_count =
		scoreboard_lifecycle_check_count(raw_jobs, common::scoreboard_is_rollback_history_job);
	let rollback_history_readback_count = raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, report)| {
			common::scoreboard_is_rollback_history_job(job) && report.status == TypedStatus::Pass
		})
		.count();

	ScoreboardLifecycleMetrics {
		stale_suppression: Some(scoreboard::ratio_or(
			stale_check_count.saturating_sub(stale_failure_count),
			stale_check_count,
			1.0,
		)),
		stale_suppressed_count: stale_check_count.saturating_sub(stale_failure_count),
		stale_check_count,
		update_correctness: Some(scoreboard::ratio_or(
			update_correct_count,
			update_check_count,
			1.0,
		)),
		update_correct_count,
		update_check_count,
		delete_correctness: Some(scoreboard::ratio_or(
			delete_correct_count,
			delete_check_count,
			1.0,
		)),
		delete_correct_count,
		delete_check_count,
		rollback_history_readback_rate: Some(scoreboard::ratio_or(
			rollback_history_readback_count,
			rollback_history_check_count,
			1.0,
		)),
		rollback_history_readback_count,
		rollback_history_check_count,
	}
}

fn scoreboard_lifecycle_check_count(
	jobs: &[RealWorldJob],
	predicate: fn(&RealWorldJob) -> bool,
) -> usize {
	jobs.iter().filter(|job| predicate(job)).count()
}

fn scoreboard_lifecycle_correct_count(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	predicate: fn(&RealWorldJob) -> bool,
) -> usize {
	raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, report)| predicate(job) && report.status == TypedStatus::Pass)
		.count()
}

fn scoreboard_answer_safety_metrics(summary: &ReportSummary) -> ScoreboardAnswerSafetyMetrics {
	ScoreboardAnswerSafetyMetrics {
		unsupported_claim_rate: Some(scoreboard::ratio(
			summary.unsupported_claim_count,
			summary.job_count,
		)),
		unsupported_claim_count: summary.unsupported_claim_count,
		stale_answer_rate: Some(scoreboard::ratio(summary.stale_answer_count, summary.job_count)),
		stale_answer_count: summary.stale_answer_count,
		hallucinated_evidence_rate: Some(summary.irrelevant_context_ratio),
		redaction_leak_count: summary.redaction_leak_count,
		irrelevant_context_ratio: Some(summary.irrelevant_context_ratio),
	}
}

fn scoreboard_operational_metrics(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
) -> ScoreboardOperationalMetrics {
	let resource_envelope_job_count = raw_jobs
		.iter()
		.filter(|job| common::scoreboard_has_any_tag(job, &["resource_envelope"]))
		.count();
	let resource_envelope_pass_count = raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, report)| {
			common::scoreboard_has_any_tag(job, &["resource_envelope"])
				&& report.status == TypedStatus::Pass
		})
		.count();

	ScoreboardOperationalMetrics {
		mean_latency_ms: summary.mean_latency_ms,
		total_cost: summary.total_cost.clone(),
		resource_envelope_status: if resource_envelope_job_count == resource_envelope_pass_count {
			"pass".to_string()
		} else {
			"typed_non_pass_present".to_string()
		},
		resource_envelope_job_count,
		resource_envelope_pass_count,
	}
}

fn elf_scoreboard_strengths(summary: &ReportSummary) -> Vec<String> {
	let mut strengths = Vec::new();

	if summary.expected_evidence_recall >= 1.0 {
		strengths.push("Expected evidence recall is complete for encoded jobs.".to_string());
	}
	if summary.source_ref_coverage >= 1.0 {
		strengths
			.push("Source-ref coverage is complete for encoded required evidence.".to_string());
	}
	if summary.stale_answer_count == 0 && summary.stale_retrieval_count == 0 {
		strengths.push("Encoded stale-answer and stale-retrieval counters are zero.".to_string());
	}
	if summary.redaction_leak_count == 0 {
		strengths.push("Encoded redaction leak count is zero.".to_string());
	}
	if summary.work_continuity.is_some() {
		strengths.push("Work Continuity readback metrics are encoded in the report.".to_string());
	}

	strengths
}
