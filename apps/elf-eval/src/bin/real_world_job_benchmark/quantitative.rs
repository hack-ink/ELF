mod audit_manifest;
mod metrics;
mod product_manifest;

pub(super) use self::{
	audit_manifest::quantitative_audit_manifest_from_jobs,
	product_manifest::quantitative_product_manifest_from_report,
};

use self::audit_manifest::{QuantitativeAuditContext, QuantitativeAuditEvidence};
use crate::{
	AdapterReport, BTreeSet, JobReport, Path, QuantitativeBenchmarkControls,
	QuantitativeBenchmarkReport, QuantitativeBenchmarkRow, RealWorldJob, ReportSummary, Result,
};

const QUANTITATIVE_SCOREBOARD_SCHEMA: &str = "elf.agent_memory_quantitative_benchmark/v1";
const QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA: &str =
	"elf.agent_memory_quantitative_product_manifest/v1";
const QUANTITATIVE_AUDIT_MANIFEST_SCHEMA: &str = "elf.agent_memory_quantitative_audit_manifest/v1";
const REQUIRED_HELD_OUT_AUDIT_CONTROL: &str = "query_ids_locked_before_product_runtime";
const REQUIRED_QREL_LEAKAGE_AUDIT_CONTROL: &str =
	"product_runtime_did_not_receive_expected_answers_or_qrels";
const REQUIRED_CANDIDATE_LEAKAGE_AUDIT_CONTROL: &str =
	"ranked_candidates_emitted_by_product_runtime";
const QUANTITATIVE_K_VALUES: &[usize] = &[1, 3, 5, 10];
const MIN_LEADERBOARD_QUERY_COUNT: usize = 30;
const WILSON_95_Z: f64 = 1.959963984540054;
const QUANTITATIVE_ROW_CLAIM_BOUNDARY: &str = concat!(
	"Quantitative metrics are bounded to this generated report. ",
	"Fixture-backed rows prove benchmark mechanics, not product-runtime or leaderboard claims."
);

pub(super) struct QuantitativeReportInput<'a> {
	pub(super) run_id: &'a str,
	pub(super) generated_at: &'a str,
	pub(super) adapter: &'a AdapterReport,
	pub(super) source_jobs: &'a [RealWorldJob],
	pub(super) jobs: &'a [JobReport],
	pub(super) summary: &'a ReportSummary,
	pub(super) product_manifest_path: Option<&'a Path>,
	pub(super) audit_manifest_path: Option<&'a Path>,
}

pub(super) fn quantitative_scoreboard_report(
	input: QuantitativeReportInput<'_>,
) -> Result<QuantitativeBenchmarkReport> {
	let corpus_id = quantitative_corpus_id(input.source_jobs);
	let evidence_class = quantitative_evidence_class(input.adapter, input.jobs);
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
	let result_state = quantitative_result_state(input.summary);
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
	let leaderboard_eligible = quantitative_row_leaderboard_eligible(
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
		suite: quantitative_suite_id(input.jobs),
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
		metrics_not_encoded: quantitative_metrics_not_encoded(
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

fn quantitative_metrics_not_encoded(
	imported_row_count: usize,
	imported_per_query_count: usize,
) -> Vec<String> {
	let mut metrics =
		vec!["paired_significance".to_string(), "audit_manifest_validation".to_string()];

	if imported_row_count == 0 {
		metrics.push("external_product_manifest_import".to_string());
	}
	if imported_row_count > 0 && imported_per_query_count == 0 {
		metrics.push("imported_product_per_query_rows".to_string());
	}

	metrics
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

fn quantitative_row_leaderboard_eligible(
	evidence_class: &str,
	sample_size: usize,
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
	metric_comparable: bool,
	audit_evidence: &QuantitativeAuditEvidence,
) -> bool {
	metric_comparable
		&& evidence_class == "live_real_world"
		&& sample_size >= MIN_LEADERBOARD_QUERY_COUNT
		&& ranking_query_count == sample_size
		&& explicit_qrel_query_count == ranking_query_count
		&& audit_evidence.held_out
		&& audit_evidence.leakage_audited
		&& audit_evidence
			.audit_manifest_id
			.as_deref()
			.is_some_and(|audit_manifest_id| !audit_manifest_id.trim().is_empty())
}
