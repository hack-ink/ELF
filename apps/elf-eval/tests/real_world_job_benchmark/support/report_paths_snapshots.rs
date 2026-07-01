use std::path::PathBuf;

use color_eyre::Result;

use crate::support;

pub(crate) fn strength_profile_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-qmd-openviking-strength-profile-report.json")
}

pub(crate) fn measurement_coverage_audit_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-measurement-coverage-audit.json")
}

pub(crate) fn retrieval_debug_profile_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-elf-qmd-retrieval-debug-profile.json")
}

pub(crate) fn trace_replay_diagnostics_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-elf-qmd-trace-replay-diagnostics-report.json")
}

pub(crate) fn competitor_strength_adoption_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-competitor-strength-adoption-report.json")
}

pub(crate) fn capture_write_policy_live_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-capture-write-policy-live-report.json")
}

pub(crate) fn live_consolidation_proposal_scoring_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-consolidation-proposal-scoring-report.json")
}

pub(crate) fn temporal_history_competitor_gap_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-temporal-history-competitor-gap-report.json")
}

pub(crate) fn dreaming_readiness_stage_ledger_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-dreaming-readiness-stage-ledger.json")
}

pub(crate) fn dreaming_competitor_strength_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-17-dreaming-competitor-strength-retest-report.json")
}

pub(crate) fn qmd_debug_ergonomics_dreaming_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.json")
}

pub(crate) fn openviking_trajectory_materialization_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-openviking-trajectory-materialization-report.json")
}

pub(crate) fn letta_core_archive_export_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-letta-core-archive-export-readback-report.json")
}

pub(crate) fn service_native_dreaming_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-service-native-dreaming-readback-report.json")
}

pub(crate) fn service_native_dreaming_readback_materialization_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-service-native-dreaming-readback-materialization.json")
}

pub(crate) fn dreaming_review_queue_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-20-dreaming-review-queue-report.json")
}

pub(crate) fn recall_debug_panel_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-20-recall-debug-panel-report.json")
}

pub(crate) fn agent_knowledge_os_closeout_benchmark_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-20-agent-knowledge-os-closeout-benchmark-report.json")
}

pub(crate) fn p2_knowledge_workspace_pageindex_openkb_closeout_report_json_path() -> Result<PathBuf>
{
	report_snapshot_path("2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.json")
}

pub(crate) fn openmemory_ui_export_product_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-openmemory-ui-export-product-readback-report.json")
}

pub(crate) fn graph_rag_citation_navigation_promotion_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-graph-rag-citation-navigation-promotion-report.json")
}

pub(crate) fn graph_rag_adapter_matrix_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-23-graph-rag-adapter-matrix-report.json")
}

pub(crate) fn p3_competitor_strength_absorption_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-23-p3-competitor-strength-absorption-report.json")
}

pub(crate) fn operator_approved_public_proxy_private_addendum_report_json_path() -> Result<PathBuf>
{
	report_snapshot_path(
		"2026-06-19-operator-approved-public-proxy-production-private-addendum.json",
	)
}

pub(crate) fn live_temporal_reconciliation_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-temporal-reconciliation-report.json")
}

pub(crate) fn competitor_strength_matrix_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-xy-897-competitor-strength-matrix.json")
}

fn report_snapshot_path(file_name: &str) -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("apps")
		.join("elf-eval")
		.join("fixtures")
		.join("report_snapshots")
		.join(file_name))
}
