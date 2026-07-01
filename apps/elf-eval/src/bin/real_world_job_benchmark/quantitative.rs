mod audit_manifest;
mod metrics;
mod product_manifest;
mod report;

pub(super) use self::{
	audit_manifest::quantitative_audit_manifest_from_jobs,
	product_manifest::quantitative_product_manifest_from_report,
	report::{QuantitativeReportInput, quantitative_scoreboard_report},
};

use self::audit_manifest::QuantitativeAuditEvidence;
use crate::{AdapterReport, BTreeSet, JobReport, RealWorldJob, ReportSummary};

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
