use crate::{
	AdapterReport, JobReport, Path, QuantitativeBenchmarkControls, QuantitativeBenchmarkReport,
	QuantitativeBenchmarkRow, RealWorldJob, ReportSummary, Result,
	quantitative::{
		self, MIN_LEADERBOARD_QUERY_COUNT, QUANTITATIVE_K_VALUES, QUANTITATIVE_ROW_CLAIM_BOUNDARY,
		QUANTITATIVE_SCOREBOARD_SCHEMA,
		audit_manifest::{self, QuantitativeAuditContext},
		metrics, product_manifest,
	},
};

pub(crate) struct QuantitativeReportInput<'a> {
	pub(crate) run_id: &'a str,
	pub(crate) generated_at: &'a str,
	pub(crate) adapter: &'a AdapterReport,
	pub(crate) source_jobs: &'a [RealWorldJob],
	pub(crate) jobs: &'a [JobReport],
	pub(crate) summary: &'a ReportSummary,
	pub(crate) product_manifest_path: Option<&'a Path>,
	pub(crate) audit_manifest_path: Option<&'a Path>,
}

pub(crate) fn quantitative_scoreboard_report(
	input: QuantitativeReportInput<'_>,
) -> Result<QuantitativeBenchmarkReport> {
	let corpus_id = quantitative::quantitative_corpus_id(input.source_jobs);
	let evidence_class = quantitative::quantitative_evidence_class(input.adapter, input.jobs);
	let per_query_rows = metrics::quantitative_per_query_rows(
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
	let result_state = quantitative::quantitative_result_state(input.summary);
	let audit_evidence = audit_manifest::quantitative_audit_evidence(
		input.audit_manifest_path,
		QuantitativeAuditContext {
			run_id: input.run_id,
			corpus_id: corpus_id.as_str(),
			product: "ELF",
			adapter_id: input.adapter.adapter_id.as_str(),
			source_jobs: input.source_jobs,
			ranking_query_count,
			explicit_qrel_query_count,
		},
	)?;
	let leaderboard_eligible = quantitative::quantitative_row_leaderboard_eligible(
		evidence_class,
		input.source_jobs.len(),
		ranking_query_count,
		explicit_qrel_query_count,
		metric_comparable,
		&audit_evidence,
	);
	let row = QuantitativeBenchmarkRow {
		product: "ELF".to_string(),
		adapter_id: input.adapter.adapter_id.clone(),
		adapter_name: input.adapter.name.clone(),
		suite: quantitative::quantitative_suite_id(input.jobs),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.clone()),
		result_state: result_state.to_string(),
		comparable: metric_comparable,
		metric_comparable,
		leaderboard_eligible,
		held_out: audit_evidence.held_out,
		leakage_audited: audit_evidence.leakage_audited,
		audit_manifest_id: audit_evidence.audit_manifest_id,
		fixture_regression_only: evidence_class == "fixture_backed",
		sample_size: input.jobs.len(),
		ranking_query_count,
		ranking_coverage_state: metrics::ranking_coverage_state(
			input.summary,
			input.source_jobs.len(),
			ranking_query_count,
		)
		.to_string(),
		ranked_candidate_source: metrics::ranked_candidate_source(ranking_query_count).to_string(),
		qrel_source: metrics::aggregate_qrel_source(ranking_query_count, explicit_qrel_query_count)
			.to_string(),
		explicit_qrel_query_count,
		metrics: metrics::aggregate_metrics(per_query_rows.as_slice()),
		metric_states: metrics::aggregate_metric_states(result_state, metric_comparable),
		denominators: metrics::aggregate_denominators(per_query_rows.as_slice()),
		confidence_intervals: metrics::aggregate_confidence_intervals(per_query_rows.as_slice()),
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	};
	let product_manifest = product_manifest::quantitative_product_manifest(
		input.product_manifest_path,
		corpus_id.as_str(),
	)?;
	let imported_row_count = product_manifest.rows.len();
	let imported_per_query_count = product_manifest.per_query_rows.len();
	let mut rows = vec![row];
	let mut merged_per_query_rows = per_query_rows;

	rows.extend(product_manifest.rows);
	merged_per_query_rows.extend(product_manifest.per_query_rows);

	let leaderboard_claim_allowed = rows.iter().filter(|row| row.leaderboard_eligible).count() >= 2;
	let controls = QuantitativeBenchmarkControls {
		same_corpus_required: true,
		same_task_required: true,
		ranked_candidates_required_for_ranking_metrics: true,
		explicit_relevance_judgments_required_for_leaderboard: true,
		minimum_query_count_for_leaderboard: MIN_LEADERBOARD_QUERY_COUNT,
		current_query_count: input.source_jobs.len(),
		current_ranking_query_count: ranking_query_count,
		current_explicit_qrel_query_count: explicit_qrel_query_count,
		leaderboard_claim_allowed,
		leakage_control:
			"held_out_or_leakage_audited_runtime_rows_required_before_leaderboard_claims"
				.to_string(),
	};

	Ok(QuantitativeBenchmarkReport {
		schema: QUANTITATIVE_SCOREBOARD_SCHEMA.to_string(),
		generated_at: input.generated_at.to_string(),
		corpus_id,
		k_values: QUANTITATIVE_K_VALUES.to_vec(),
		rows,
		per_query_rows: merged_per_query_rows,
		metrics_not_encoded: quantitative::quantitative_metrics_not_encoded(
			imported_row_count,
			imported_per_query_count,
		),
		controls,
		claim_boundary: concat!(
			"Do not convert fixture mechanics, missing explicit qrels, ",
			"or partial candidate coverage into product leaderboard claims."
		)
		.to_string(),
	})
}
