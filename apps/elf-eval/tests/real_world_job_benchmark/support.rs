use std::{
	env, fs,
	path::{Path, PathBuf},
	process::{self, Command, Output},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

pub(super) struct RecallDebugSourceContract<'a> {
	pub(super) service: &'a str,
	pub(super) service_lib: &'a str,
	pub(super) routes: &'a str,
	pub(super) mcp: &'a str,
	pub(super) recall_spec: &'a str,
	pub(super) service_spec: &'a str,
	pub(super) version_registry: &'a str,
	pub(super) markdown: &'a str,
	pub(super) benchmarking_index: &'a str,
	pub(super) readme: &'a str,
}

pub(super) fn fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_memory")
		.join("work_resume")
}

pub(super) fn fixture_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_memory")
}

pub(super) fn real_world_memory_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_memory")
}

pub(super) fn evolution_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("evolution")
}

pub(super) fn operator_debug_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_job")
		.join("operator_debugging_ux")
}

pub(super) fn project_decisions_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("project_decisions")
}

pub(super) fn retrieval_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_memory")
		.join("retrieval")
}

pub(super) fn capture_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("capture_integration")
}

pub(super) fn consolidation_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("consolidation")
}

pub(super) fn memory_summary_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("memory_summary")
}

pub(super) fn proactive_brief_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("proactive_brief")
}

pub(super) fn scheduled_memory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("scheduled_memory")
}

pub(super) fn work_continuity_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("work_continuity")
}

pub(super) fn knowledge_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("knowledge")
}

pub(super) fn source_library_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("source_library")
}

pub(super) fn production_ops_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("production_ops")
}

pub(super) fn core_archival_memory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("core_archival_memory")
}

pub(super) fn context_trajectory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("context_trajectory")
}

pub(super) fn adversarial_quality_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("adversarial_quality")
}

pub(super) fn graph_rag_external_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("graph_rag")
}

pub(super) fn workspace_root() -> Result<PathBuf> {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let root = manifest_dir
		.parent()
		.and_then(Path::parent)
		.ok_or_else(|| eyre::eyre!("could not resolve workspace root"))?;

	Ok(root.to_path_buf())
}

pub(super) fn collapse_whitespace(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn report_snapshot_path(file_name: &str) -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("apps")
		.join("elf-eval")
		.join("fixtures")
		.join("report_snapshots")
		.join(file_name))
}

pub(super) fn strength_profile_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-qmd-openviking-strength-profile-report.json")
}

pub(super) fn strength_profile_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-qmd-openviking-strength-profile-report.md"))
}

pub(super) fn measurement_coverage_audit_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-measurement-coverage-audit.md"))
}

pub(super) fn measurement_coverage_audit_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-measurement-coverage-audit.json")
}

pub(super) fn retrieval_debug_profile_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-elf-qmd-retrieval-debug-profile.json")
}

pub(super) fn trace_replay_diagnostics_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-elf-qmd-trace-replay-diagnostics-report.json")
}

pub(super) fn trace_replay_diagnostics_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"))
}

pub(super) fn competitor_strength_adoption_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-adoption-report.md"))
}

pub(super) fn competitor_strength_adoption_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-competitor-strength-adoption-report.json")
}

pub(super) fn capture_write_policy_live_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-capture-write-policy-live-report.json")
}

pub(super) fn capture_write_policy_live_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-capture-write-policy-live-report.md"))
}

pub(super) fn live_consolidation_proposal_scoring_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-consolidation-proposal-scoring-report.json")
}

pub(super) fn live_consolidation_proposal_scoring_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-live-consolidation-proposal-scoring-report.md"))
}

pub(super) fn temporal_history_competitor_gap_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-temporal-history-competitor-gap-report.json")
}

pub(super) fn dreaming_readiness_stage_ledger_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-dreaming-readiness-stage-ledger.json")
}

pub(super) fn dreaming_readiness_stage_ledger_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-dreaming-readiness-stage-ledger.md"))
}

pub(super) fn dreaming_competitor_strength_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-17-dreaming-competitor-strength-retest-report.json")
}

pub(super) fn dreaming_competitor_strength_retest_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-17-dreaming-competitor-strength-retest-report.md"))
}

pub(super) fn qmd_debug_ergonomics_dreaming_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.json")
}

pub(super) fn qmd_debug_ergonomics_dreaming_retest_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md"))
}

pub(super) fn openviking_trajectory_materialization_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-openviking-trajectory-materialization-report.json")
}

pub(super) fn letta_core_archive_export_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-letta-core-archive-export-readback-report.json")
}

pub(super) fn service_native_dreaming_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-service-native-dreaming-readback-report.json")
}

pub(super) fn service_native_dreaming_readback_materialization_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-service-native-dreaming-readback-materialization.json")
}

pub(super) fn dreaming_review_queue_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-20-dreaming-review-queue-report.json")
}

pub(super) fn recall_debug_panel_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-20-recall-debug-panel-report.json")
}

pub(super) fn agent_knowledge_os_closeout_benchmark_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-20-agent-knowledge-os-closeout-benchmark-report.json")
}

pub(super) fn p2_knowledge_workspace_pageindex_openkb_closeout_report_json_path() -> Result<PathBuf>
{
	report_snapshot_path("2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.json")
}

pub(super) fn openmemory_ui_export_product_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-openmemory-ui-export-product-readback-report.json")
}

pub(super) fn graph_rag_citation_navigation_promotion_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-graph-rag-citation-navigation-promotion-report.json")
}

pub(super) fn graph_rag_adapter_matrix_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-23-graph-rag-adapter-matrix-report.json")
}

pub(super) fn p3_competitor_strength_absorption_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-23-p3-competitor-strength-absorption-report.json")
}

pub(super) fn operator_approved_public_proxy_private_addendum_report_json_path() -> Result<PathBuf>
{
	report_snapshot_path(
		"2026-06-19-operator-approved-public-proxy-production-private-addendum.json",
	)
}

pub(super) fn openviking_trajectory_materialization_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-openviking-trajectory-materialization-report.md"))
}

pub(super) fn letta_core_archive_export_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-letta-core-archive-export-readback-report.md"))
}

pub(super) fn service_native_dreaming_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-service-native-dreaming-readback-report.md"))
}

pub(super) fn dreaming_review_queue_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-dreaming-review-queue-report.md"))
}

pub(super) fn recall_debug_panel_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-recall-debug-panel-report.md"))
}

pub(super) fn agent_knowledge_os_closeout_benchmark_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-agent-knowledge-os-closeout-benchmark-report.md"))
}

pub(super) fn p2_knowledge_workspace_pageindex_openkb_closeout_report_markdown_path()
-> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md"))
}

pub(super) fn openmemory_ui_export_product_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-openmemory-ui-export-product-readback-report.md"))
}

pub(super) fn graph_rag_citation_navigation_promotion_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-graph-rag-citation-navigation-promotion-report.md"))
}

pub(super) fn graph_rag_adapter_matrix_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-23-graph-rag-adapter-matrix-report.md"))
}

pub(super) fn p3_competitor_strength_absorption_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-23-p3-competitor-strength-absorption-report.md"))
}

pub(super) fn graph_topic_map_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-20-graph-topic-map-report.md"))
}

pub(super) fn operator_approved_public_proxy_private_addendum_report_markdown_path()
-> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-operator-approved-public-proxy-production-private-addendum.md"))
}

pub(super) fn live_temporal_reconciliation_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-temporal-reconciliation-report.json")
}

pub(super) fn live_temporal_reconciliation_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-live-temporal-reconciliation-report.md"))
}

pub(super) fn competitor_strength_matrix_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-evidence-matrix.md"))
}

pub(super) fn competitor_strength_matrix_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-xy-897-competitor-strength-matrix.json")
}

pub(super) fn readme_path() -> Result<PathBuf> {
	Ok(workspace_root()?.join("README.md"))
}

pub(super) fn comparison_external_projects_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("external_memory")
		.join("comparison_external_projects.md"))
}

pub(super) fn benchmarking_index_path() -> Result<PathBuf> {
	Ok(workspace_root()?.join("docs").join("evidence").join("benchmarking").join("index.md"))
}

pub(super) fn iteration_direction_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-elf-iteration-direction-from-competitor-benchmarks.md"))
}

pub(super) fn external_adapter_manifest_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("memory_projects_manifest.json")
}

pub(super) fn run_json_report_from(fixtures: PathBuf) -> Result<Value> {
	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixtures)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	Ok(serde_json::from_slice(&output.stdout)?)
}

pub(super) fn run_json_report_from_failure(fixtures: PathBuf) -> Result<String> {
	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixtures)
		.output()?;

	assert!(
		!output.status.success(),
		"real_world_job runner unexpectedly passed: {}",
		String::from_utf8_lossy(&output.stdout),
	);

	Ok(String::from_utf8_lossy(&output.stderr).to_string())
}

pub(super) fn run_json_report() -> Result<Value> {
	run_json_report_from(fixture_dir())
}

pub(super) fn load_json(path: &Path) -> Result<Value> {
	Ok(serde_json::from_str::<Value>(&fs::read_to_string(path)?)?)
}

pub(super) fn array_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
	value
		.pointer(pointer)
		.and_then(Value::as_array)
		.ok_or_else(|| eyre::eyre!("missing array at {pointer}"))
}

pub(super) fn find_by_field<'a>(
	items: &'a [Value],
	field: &str,
	expected: &str,
) -> Result<&'a Value> {
	items
		.iter()
		.find(|item| item.pointer(field).and_then(Value::as_str) == Some(expected))
		.ok_or_else(|| eyre::eyre!("missing item with {field} = {expected}"))
}

pub(super) fn array_contains_str(value: &Value, pointer: &str, expected: &str) -> Result<bool> {
	Ok(array_at(value, pointer)?.iter().any(|item| item.as_str() == Some(expected)))
}

pub(super) fn string_array_at(value: &Value, pointer: &str) -> Result<Vec<String>> {
	array_at(value, pointer)?
		.iter()
		.map(|item| {
			item.as_str()
				.map(str::to_owned)
				.ok_or_else(|| eyre::eyre!("non-string entry at {pointer}"))
		})
		.collect()
}

pub(super) fn set_json_pointer(value: &mut Value, pointer: &str, replacement: Value) -> Result<()> {
	let target =
		value.pointer_mut(pointer).ok_or_else(|| eyre::eyre!("missing JSON pointer {pointer}"))?;

	*target = replacement;

	Ok(())
}

pub(super) fn run_external_manifest_with_letta_attachment_mutation<F>(
	slug: &str,
	mutation: F,
) -> Result<Output>
where
	F: FnOnce(&mut Value) -> Result<()>,
{
	run_external_manifest_scenario_mutation(
		slug,
		"letta_research_gate",
		"core_block_attachment_readback",
		mutation,
	)
}

pub(super) fn run_external_manifest_scenario_mutation<F>(
	slug: &str,
	adapter_id: &str,
	scenario_id: &str,
	mutation: F,
) -> Result<Output>
where
	F: FnOnce(&mut Value) -> Result<()>,
{
	let mut manifest =
		serde_json::from_str::<Value>(&fs::read_to_string(external_adapter_manifest_path())?)?;
	let adapters = manifest
		.pointer_mut("/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing manifest adapters"))?;
	let adapter = adapters
		.iter_mut()
		.find(|adapter| adapter.pointer("/adapter_id").and_then(Value::as_str) == Some(adapter_id))
		.ok_or_else(|| eyre::eyre!("missing {adapter_id} adapter"))?;
	let scenarios = adapter
		.pointer_mut("/scenarios")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing {adapter_id} scenarios"))?;
	let scenario = scenarios
		.iter_mut()
		.find(|scenario| {
			scenario.pointer("/scenario_id").and_then(Value::as_str) == Some(scenario_id)
		})
		.ok_or_else(|| eyre::eyre!("missing {scenario_id} scenario"))?;

	mutation(scenario)?;

	let temp_dir = env::temp_dir().join(format!("elf-real-world-{slug}-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let manifest_path = temp_dir.join("memory_projects_manifest.json");

	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	Ok(Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixture_dir())
		.arg("--external-adapter-manifest")
		.arg(&manifest_path)
		.output()?)
}
