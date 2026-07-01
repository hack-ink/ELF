use std::path::PathBuf;

use color_eyre::Result;

use crate::support;

pub(crate) fn report_snapshot_path(file_name: &str) -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("apps")
		.join("elf-eval")
		.join("fixtures")
		.join("report_snapshots")
		.join(file_name))
}

pub(crate) fn strength_profile_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-qmd-openviking-strength-profile-report.json")
}

pub(crate) fn strength_profile_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-qmd-openviking-strength-profile-report.md"))
}

pub(crate) fn measurement_coverage_audit_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-measurement-coverage-audit.md"))
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

pub(crate) fn trace_replay_diagnostics_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"))
}

pub(crate) fn competitor_strength_adoption_report_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-adoption-report.md"))
}

pub(crate) fn competitor_strength_adoption_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-competitor-strength-adoption-report.json")
}

pub(crate) fn capture_write_policy_live_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-capture-write-policy-live-report.json")
}

pub(crate) fn capture_write_policy_live_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-capture-write-policy-live-report.md"))
}

pub(crate) fn live_consolidation_proposal_scoring_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-consolidation-proposal-scoring-report.json")
}

pub(crate) fn live_consolidation_proposal_scoring_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-live-consolidation-proposal-scoring-report.md"))
}

pub(crate) fn temporal_history_competitor_gap_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-temporal-history-competitor-gap-report.json")
}

pub(crate) fn dreaming_readiness_stage_ledger_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-dreaming-readiness-stage-ledger.json")
}

pub(crate) fn dreaming_readiness_stage_ledger_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-dreaming-readiness-stage-ledger.md"))
}

pub(crate) fn dreaming_competitor_strength_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-17-dreaming-competitor-strength-retest-report.json")
}

pub(crate) fn dreaming_competitor_strength_retest_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-17-dreaming-competitor-strength-retest-report.md"))
}

pub(crate) fn qmd_debug_ergonomics_dreaming_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.json")
}

pub(crate) fn qmd_debug_ergonomics_dreaming_retest_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md"))
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

pub(crate) fn openviking_trajectory_materialization_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-openviking-trajectory-materialization-report.md"))
}

pub(crate) fn letta_core_archive_export_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-letta-core-archive-export-readback-report.md"))
}

pub(crate) fn service_native_dreaming_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-service-native-dreaming-readback-report.md"))
}

pub(crate) fn dreaming_review_queue_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-dreaming-review-queue-report.md"))
}

pub(crate) fn recall_debug_panel_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-recall-debug-panel-report.md"))
}

pub(crate) fn agent_knowledge_os_closeout_benchmark_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-agent-knowledge-os-closeout-benchmark-report.md"))
}

pub(crate) fn p2_knowledge_workspace_pageindex_openkb_closeout_report_markdown_path()
-> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md"))
}

pub(crate) fn openmemory_ui_export_product_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-openmemory-ui-export-product-readback-report.md"))
}

pub(crate) fn graph_rag_citation_navigation_promotion_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-graph-rag-citation-navigation-promotion-report.md"))
}

pub(crate) fn graph_rag_adapter_matrix_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-23-graph-rag-adapter-matrix-report.md"))
}

pub(crate) fn p3_competitor_strength_absorption_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-23-p3-competitor-strength-absorption-report.md"))
}

pub(crate) fn graph_topic_map_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-graph-topic-map-report.md"))
}

pub(crate) fn operator_approved_public_proxy_private_addendum_report_markdown_path()
-> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-operator-approved-public-proxy-production-private-addendum.md"))
}

pub(crate) fn live_temporal_reconciliation_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-temporal-reconciliation-report.json")
}

pub(crate) fn live_temporal_reconciliation_report_markdown_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-live-temporal-reconciliation-report.md"))
}

pub(crate) fn competitor_strength_matrix_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-evidence-matrix.md"))
}

pub(crate) fn competitor_strength_matrix_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-xy-897-competitor-strength-matrix.json")
}

pub(crate) fn readme_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?.join("README.md"))
}

pub(crate) fn comparison_external_projects_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("external_memory")
		.join("comparison_external_projects.md"))
}

pub(crate) fn benchmarking_index_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("index.md"))
}

pub(crate) fn iteration_direction_report_path() -> Result<PathBuf> {
	Ok(support::workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-elf-iteration-direction-from-competitor-benchmarks.md"))
}
