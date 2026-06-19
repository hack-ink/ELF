#![allow(unused_crate_dependencies)]

//! Integration tests for the real-world job smoke benchmark runner.

use std::{
	env, fs,
	path::{Path, PathBuf},
	process::{self, Command, Output},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

fn fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_memory")
		.join("work_resume")
}

fn fixture_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_memory")
}

fn real_world_memory_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("real_world_memory")
}

fn evolution_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("evolution")
}

fn operator_debug_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_job")
		.join("operator_debugging_ux")
}

fn project_decisions_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("project_decisions")
}

fn retrieval_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_memory")
		.join("retrieval")
}

fn capture_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("capture_integration")
}

fn consolidation_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("consolidation")
}

fn memory_summary_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("memory_summary")
}

fn proactive_brief_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("proactive_brief")
}

fn scheduled_memory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("scheduled_memory")
}

fn knowledge_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("knowledge")
}

fn source_library_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("source_library")
}

fn production_ops_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("production_ops")
}

fn core_archival_memory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("core_archival_memory")
}

fn context_trajectory_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("context_trajectory")
}

fn graph_rag_external_fixture_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("graph_rag")
}

fn workspace_root() -> Result<PathBuf> {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let root = manifest_dir
		.parent()
		.and_then(Path::parent)
		.ok_or_else(|| eyre::eyre!("could not resolve workspace root"))?;

	Ok(root.to_path_buf())
}

fn collapse_whitespace(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn report_snapshot_path(file_name: &str) -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("apps")
		.join("elf-eval")
		.join("fixtures")
		.join("report_snapshots")
		.join(file_name))
}

fn strength_profile_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-qmd-openviking-strength-profile-report.json")
}

fn strength_profile_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-qmd-openviking-strength-profile-report.md"))
}

fn measurement_coverage_audit_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-measurement-coverage-audit.md"))
}

fn measurement_coverage_audit_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-measurement-coverage-audit.json")
}

fn retrieval_debug_profile_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-elf-qmd-retrieval-debug-profile.json")
}

fn trace_replay_diagnostics_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-elf-qmd-trace-replay-diagnostics-report.json")
}

fn trace_replay_diagnostics_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"))
}

fn competitor_strength_adoption_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-adoption-report.md"))
}

fn competitor_strength_adoption_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-competitor-strength-adoption-report.json")
}

fn capture_write_policy_live_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-capture-write-policy-live-report.json")
}

fn capture_write_policy_live_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-capture-write-policy-live-report.md"))
}

fn live_consolidation_proposal_scoring_report_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-consolidation-proposal-scoring-report.json")
}

fn live_consolidation_proposal_scoring_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-live-consolidation-proposal-scoring-report.md"))
}

fn temporal_history_competitor_gap_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-temporal-history-competitor-gap-report.json")
}

fn dreaming_readiness_stage_ledger_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-dreaming-readiness-stage-ledger.json")
}

fn dreaming_readiness_stage_ledger_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-dreaming-readiness-stage-ledger.md"))
}

fn dreaming_competitor_strength_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-17-dreaming-competitor-strength-retest-report.json")
}

fn dreaming_competitor_strength_retest_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-17-dreaming-competitor-strength-retest-report.md"))
}

fn qmd_debug_ergonomics_dreaming_retest_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.json")
}

fn qmd_debug_ergonomics_dreaming_retest_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md"))
}

fn openviking_trajectory_materialization_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-openviking-trajectory-materialization-report.json")
}

fn letta_core_archive_export_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-letta-core-archive-export-readback-report.json")
}

fn service_native_dreaming_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-service-native-dreaming-readback-report.json")
}

fn service_native_dreaming_readback_materialization_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-service-native-dreaming-readback-materialization.json")
}

fn openmemory_ui_export_product_readback_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-openmemory-ui-export-product-readback-report.json")
}

fn graph_rag_citation_navigation_promotion_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-19-graph-rag-citation-navigation-promotion-report.json")
}

fn operator_approved_public_proxy_private_addendum_report_json_path() -> Result<PathBuf> {
	report_snapshot_path(
		"2026-06-19-operator-approved-public-proxy-production-private-addendum.json",
	)
}

fn openviking_trajectory_materialization_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-openviking-trajectory-materialization-report.md"))
}

fn letta_core_archive_export_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-letta-core-archive-export-readback-report.md"))
}

fn service_native_dreaming_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-service-native-dreaming-readback-report.md"))
}

fn openmemory_ui_export_product_readback_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-openmemory-ui-export-product-readback-report.md"))
}

fn graph_rag_citation_navigation_promotion_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-graph-rag-citation-navigation-promotion-report.md"))
}

fn operator_approved_public_proxy_private_addendum_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-19-operator-approved-public-proxy-production-private-addendum.md"))
}

fn live_temporal_reconciliation_report_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-16-live-temporal-reconciliation-report.json")
}

fn live_temporal_reconciliation_report_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-16-live-temporal-reconciliation-report.md"))
}

fn competitor_strength_matrix_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-evidence-matrix.md"))
}

fn competitor_strength_matrix_json_path() -> Result<PathBuf> {
	report_snapshot_path("2026-06-11-xy-897-competitor-strength-matrix.json")
}

fn readme_path() -> Result<PathBuf> {
	Ok(workspace_root()?.join("README.md"))
}

fn comparison_external_projects_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("external_memory")
		.join("comparison_external_projects.md"))
}

fn benchmarking_index_path() -> Result<PathBuf> {
	Ok(workspace_root()?.join("docs").join("evidence").join("benchmarking").join("index.md"))
}

fn iteration_direction_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("evidence")
		.join("benchmarking")
		.join("2026-06-11-elf-iteration-direction-from-competitor-benchmarks.md"))
}

fn external_adapter_manifest_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("memory_projects_manifest.json")
}

fn run_json_report_from(fixtures: PathBuf) -> Result<Value> {
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

fn run_json_report() -> Result<Value> {
	run_json_report_from(fixture_dir())
}

fn load_json(path: &Path) -> Result<Value> {
	Ok(serde_json::from_str::<Value>(&fs::read_to_string(path)?)?)
}

fn array_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
	value
		.pointer(pointer)
		.and_then(Value::as_array)
		.ok_or_else(|| eyre::eyre!("missing array at {pointer}"))
}

fn find_by_field<'a>(items: &'a [Value], field: &str, expected: &str) -> Result<&'a Value> {
	items
		.iter()
		.find(|item| item.pointer(field).and_then(Value::as_str) == Some(expected))
		.ok_or_else(|| eyre::eyre!("missing item with {field} = {expected}"))
}

fn array_contains_str(value: &Value, pointer: &str, expected: &str) -> Result<bool> {
	Ok(array_at(value, pointer)?.iter().any(|item| item.as_str() == Some(expected)))
}

fn string_array_at(value: &Value, pointer: &str) -> Result<Vec<String>> {
	array_at(value, pointer)?
		.iter()
		.map(|item| {
			item.as_str()
				.map(str::to_owned)
				.ok_or_else(|| eyre::eyre!("non-string entry at {pointer}"))
		})
		.collect()
}

fn set_json_pointer(value: &mut Value, pointer: &str, replacement: Value) -> Result<()> {
	let target =
		value.pointer_mut(pointer).ok_or_else(|| eyre::eyre!("missing JSON pointer {pointer}"))?;

	*target = replacement;

	Ok(())
}

fn run_external_manifest_with_letta_attachment_mutation<F>(
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

fn run_external_manifest_scenario_mutation<F>(
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

#[test]
fn smoke_fixture_produces_typed_json_report() -> Result<()> {
	let report = run_json_report()?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.real_world_job_report/v1")
	);
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/external_adapters/summary/adapter_count").and_then(Value::as_u64),
		Some(23)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/live_real_world_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(11)
	);

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "work-resume-stale-worktree-001")?;

	assert_eq!(job.pointer("/suite_id").and_then(Value::as_str), Some("work_resume"));
	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(job.pointer("/latency_ms").and_then(Value::as_f64), Some(2.0));
	assert_eq!(job.pointer("/cost/amount").and_then(Value::as_f64), Some(0.0));

	let expected_evidence = array_at(job, "/expected_evidence")?;
	let produced_evidence = array_at(job, "/produced_evidence")?;

	assert_eq!(expected_evidence.len(), 2);
	assert_eq!(produced_evidence.len(), 1);
	assert_eq!(produced_evidence.first().and_then(Value::as_str), Some("xy844-current-worktree"));

	let suites = array_at(&report, "/suites")?;
	let encoded_suite = find_by_field(suites, "/suite_id", "work_resume")?;
	let capture_suite = find_by_field(suites, "/suite_id", "capture_integration")?;
	let unencoded_suite = find_by_field(suites, "/suite_id", "retrieval")?;

	assert_eq!(encoded_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(encoded_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(capture_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(capture_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(unencoded_suite.pointer("/status").and_then(Value::as_str), Some("not_encoded"));

	let capture_fixture_backed = array_at(&report, "/capture_integration/fixture_backed")?;

	assert!(capture_fixture_backed.iter().any(|value| {
		value.as_str().is_some_and(|item| item.contains("agentmemory-style hook capture"))
	}));

	let capture_not_encoded = array_at(&report, "/capture_integration/not_encoded")?;

	assert!(capture_not_encoded.iter().any(|value| {
		value.as_str().is_some_and(|item| item.contains("No live external hook ingestion"))
	}));

	Ok(())
}

#[test]
fn real_world_report_includes_external_adapter_coverage_manifest() -> Result<()> {
	let report = run_json_report_from(real_world_memory_fixture_dir())?;

	assert_external_adapter_manifest_summary(&report);
	assert_external_adapter_manifest_records(&report)?;

	Ok(())
}

#[test]
fn capture_integration_fixtures_score_redaction_and_source_ids() -> Result<()> {
	let report = run_json_report_from(capture_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));

	let suites = array_at(&report, "/suites")?;
	let capture = find_by_field(suites, "/suite_id", "capture_integration")?;

	assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(capture.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = array_at(&report, "/jobs")?;
	let source_id = find_by_field(jobs, "/job_id", "capture-source-id-binding-001")?;
	let redaction = find_by_field(jobs, "/job_id", "capture-write-policy-redaction-001")?;

	assert!(array_contains_str(source_id, "/produced_evidence", "source-id-release-summary")?);
	assert!(array_contains_str(source_id, "/produced_evidence", "source-id-command-log")?);
	assert_eq!(redaction.pointer("/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert!(
		redaction
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| !answer.contains("orchid-envelope"))
	);

	Ok(())
}

#[test]
fn source_library_fixtures_score_saved_sources_without_memory_promotion() -> Result<()> {
	let report = run_json_report_from(source_library_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));

	let suites = array_at(&report, "/suites")?;
	let source_library = find_by_field(suites, "/suite_id", "source_library")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/encoded_job_count").and_then(Value::as_u64), Some(2));

	let jobs = array_at(&report, "/jobs")?;
	let long_doc = find_by_field(jobs, "/job_id", "source-library-long-doc-001")?;
	let thread = find_by_field(jobs, "/job_id", "source-library-social-thread-001")?;

	assert!(array_contains_str(long_doc, "/produced_evidence", "article-source-record")?);
	assert!(array_contains_str(long_doc, "/produced_evidence", "article-hydrated-excerpt")?);
	assert!(array_contains_str(thread, "/produced_evidence", "thread-source-record")?);
	assert!(array_contains_str(thread, "/produced_evidence", "thread-promotion-boundary")?);
	assert!(long_doc.pointer("/produced_answer").and_then(Value::as_str).is_some_and(|answer| {
		answer.contains("does not automatically create a durable Memory Note")
	}));
	assert!(
		thread
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| answer.contains("explicit add_note or reviewed promotion"))
	);

	Ok(())
}

#[test]
fn external_adapter_run_summarizes_nonzero_scenario_losses() -> Result<()> {
	let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("memory_projects_manifest.json");
	let mut manifest = serde_json::from_str::<Value>(&fs::read_to_string(manifest_path)?)?;
	let adapters = manifest
		.pointer_mut("/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing manifest adapters"))?;
	let adapter = adapters
		.iter_mut()
		.find(|adapter| {
			adapter.pointer("/adapter_id").and_then(Value::as_str)
				== Some("agentmemory_live_baseline")
		})
		.ok_or_else(|| eyre::eyre!("missing agentmemory adapter"))?;

	set_json_pointer(adapter, "/scenarios/0/elf_position", serde_json::json!("loses"))?;
	set_json_pointer(adapter, "/scenarios/0/comparison_outcome", serde_json::json!("loss"))?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-loss-manifest-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let manifest_path = temp_dir.join("memory_projects_manifest.json");

	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixture_dir())
		.arg("--external-adapter-manifest")
		.arg(&manifest_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let report = serde_json::from_slice::<Value>(&output.stdout)?;

	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/loses")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/untested")
			.and_then(Value::as_u64),
		Some(34)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/loss")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/not_tested")
			.and_then(Value::as_u64),
		Some(12)
	);

	let adapters = array_at(&report, "/external_adapters/adapters")?;
	let agentmemory = find_by_field(adapters, "/adapter_id", "agentmemory_live_baseline")?;

	assert_eq!(
		agentmemory.pointer("/scenarios/0/elf_position").and_then(Value::as_str),
		Some("loses")
	);

	Ok(())
}

fn assert_external_adapter_manifest_summary(report: &Value) {
	assert_eq!(
		report.pointer("/external_adapters/schema").and_then(Value::as_str),
		Some("elf.real_world_external_adapter_report/v1")
	);
	assert_eq!(
		report.pointer("/external_adapters/manifest_id").and_then(Value::as_str),
		Some(
			"real-world-memory-project-adapters-2026-06-11-first-generation-continuity-source-store"
		)
	);
	assert_eq!(
		report.pointer("/external_adapters/docker_isolation/default").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/docker_isolation/host_global_installs_required")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/adapter_count").and_then(Value::as_u64),
		Some(23)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/external_project_count").and_then(Value::as_u64),
		Some(16)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/fixture_backed_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/live_baseline_only_count")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/live_real_world_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(11)
	);

	assert_external_adapter_manifest_status_summary(report);
	assert_external_adapter_manifest_scenario_summary(report);
}

fn assert_external_adapter_manifest_status_summary(report: &Value) {
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/pass")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/lifecycle_fail")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/incomplete")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/blocked")
			.and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/capability_status_counts/mocked")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/capability_status_counts/unsupported")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/suite_status_counts/blocked")
			.and_then(Value::as_u64),
		Some(24)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/suite_status_counts/pass")
			.and_then(Value::as_u64),
		Some(27)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/suite_status_counts/incomplete")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/suite_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(37)
	);
}

fn assert_external_adapter_manifest_scenario_summary(report: &Value) {
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/real")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/mocked")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/unsupported")
			.and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/blocked")
			.and_then(Value::as_u64),
		Some(16)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/incomplete")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/lifecycle_fail")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/pass")
			.and_then(Value::as_u64),
		Some(23)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/wins")
			.and_then(Value::as_u64),
		Some(10)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/ties")
			.and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/loses")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/untested")
			.and_then(Value::as_u64),
		Some(35)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/win")
			.and_then(Value::as_u64),
		Some(10)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/tie")
			.and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/loss")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/not_tested")
			.and_then(Value::as_u64),
		Some(13)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/blocked")
			.and_then(Value::as_u64),
		Some(17)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/non_goal")
			.and_then(Value::as_u64),
		Some(5)
	);
}

fn assert_external_adapter_manifest_records(report: &Value) -> Result<()> {
	let adapters = array_at(report, "/external_adapters/adapters")?;
	let elf = find_by_field(adapters, "/adapter_id", "elf_real_world_memory_fixture")?;
	let elf_live = find_by_field(adapters, "/adapter_id", "elf_live_real_world")?;
	let elf_operator_debug = find_by_field(adapters, "/adapter_id", "elf_operator_debug_live")?;
	let qmd = find_by_field(adapters, "/adapter_id", "qmd_live_baseline")?;
	let qmd_live = find_by_field(adapters, "/adapter_id", "qmd_live_real_world")?;
	let qmd_operator_debug = find_by_field(adapters, "/adapter_id", "qmd_operator_debug_live")?;
	let agentmemory = find_by_field(adapters, "/adapter_id", "agentmemory_live_baseline")?;
	let mem0 = find_by_field(adapters, "/adapter_id", "mem0_openmemory_live_baseline")?;
	let memsearch = find_by_field(adapters, "/adapter_id", "memsearch_live_baseline")?;
	let openviking = find_by_field(adapters, "/adapter_id", "openviking_live_baseline")?;
	let claude_mem = find_by_field(adapters, "/adapter_id", "claude_mem_live_baseline")?;
	let ragflow = find_by_field(adapters, "/adapter_id", "ragflow_research_gate")?;
	let lightrag = find_by_field(adapters, "/adapter_id", "lightrag_research_gate")?;
	let graphrag = find_by_field(adapters, "/adapter_id", "graphrag_research_gate")?;
	let graphiti_zep = find_by_field(adapters, "/adapter_id", "graphiti_zep_research_gate")?;
	let graphify = find_by_field(adapters, "/adapter_id", "graphify_docker_smoke")?;
	let qmd_deep = find_by_field(adapters, "/adapter_id", "qmd_deep_profile_gate")?;
	let openviking_deep = find_by_field(adapters, "/adapter_id", "openviking_deep_profile_gate")?;
	let letta = find_by_field(adapters, "/adapter_id", "letta_research_gate")?;

	assert_elf_fixture_adapter_record(elf)?;

	assert_eq!(
		elf_live.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(elf_live.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));

	assert_live_sweep_record(elf_live, "blocked")?;
	assert_operator_debug_live_adapter_records(elf_operator_debug, qmd_operator_debug)?;

	assert_eq!(qmd.pointer("/overall_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(qmd.pointer("/suites/0/status").and_then(Value::as_str), Some("not_encoded"));

	assert_qmd_live_baseline_record(qmd);

	assert_eq!(
		qmd_live.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(qmd_live.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));

	assert_live_sweep_record(qmd_live, "blocked")?;

	assert_eq!(
		agentmemory.pointer("/capabilities/1/status").and_then(Value::as_str),
		Some("mocked")
	);

	assert_first_generation_adapter_records(agentmemory, mem0, memsearch, claude_mem);

	assert_eq!(openviking.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));

	assert_graph_rag_research_gate_records(ragflow, lightrag, graphrag);
	assert_graphiti_zep_adapter(graphiti_zep);
	assert_graphify_adapter(graphify)?;
	assert_graph_rag_representative_scenarios(ragflow, lightrag, graphrag, graphiti_zep, graphify)?;
	assert_letta_core_archival_gate(letta)?;
	assert_qmd_deep_profile_gate(qmd_deep);

	assert_eq!(
		qmd_deep.pointer("/capabilities/2/status").and_then(Value::as_str),
		Some("unsupported")
	);
	assert_eq!(
		qmd_deep.pointer("/result/artifact").and_then(Value::as_str),
		Some("docs/evidence/benchmarking/2026-06-11-qmd-openviking-strength-profile-report.md")
	);
	assert_eq!(
		openviking_deep.pointer("/adapter_kind").and_then(Value::as_str),
		Some("docker_local_embed_context_trajectory_gate")
	);

	assert_openviking_deep_profile_gate(openviking_deep);

	assert_eq!(
		openviking_deep.pointer("/result/artifact").and_then(Value::as_str),
		Some("docs/evidence/benchmarking/2026-06-11-qmd-openviking-strength-profile-report.md")
	);

	Ok(())
}

fn assert_graph_rag_research_gate_records(ragflow: &Value, lightrag: &Value, graphrag: &Value) {
	assert_eq!(ragflow.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(ragflow.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		ragflow.pointer("/execution_metadata/research_depth").and_then(Value::as_str),
		Some(
			"D2 feasibility verdict plus XY-885 evidence-smoke implementation and XY-900 scored smoke promotion; checked-in record remains research_gate unless a generated artifact reaches query output"
		)
	);
	assert_eq!(
		ragflow.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-ragflow-docker")
	);
	assert_eq!(
		ragflow.pointer("/result/artifact").and_then(Value::as_str),
		Some("tmp/real-world-memory/ragflow-smoke/ragflow-report.json")
	);
	assert_eq!(
		ragflow.pointer("/execution_metadata/sources/0/url").and_then(Value::as_str),
		Some("https://github.com/infiniflow/ragflow")
	);
	assert_eq!(lightrag.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(lightrag.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		lightrag.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-lightrag-docker-context")
	);
	assert_eq!(
		lightrag.pointer("/run/command").and_then(Value::as_str),
		Some("ELF_LIGHTRAG_CONTEXT_START=1 cargo make smoke-lightrag-docker-context")
	);
	assert_eq!(
		lightrag.pointer("/capabilities/3/status").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(graphrag.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(
		graphrag.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-graphrag-docker")
	);
	assert_eq!(graphrag.pointer("/suites/1/status").and_then(Value::as_str), Some("not_encoded"));
}

fn assert_letta_core_archival_gate(adapter: &Value) -> Result<()> {
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert!(
		adapter
			.pointer("/setup/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("smoke-letta-core-archive-export-readback")
				&& evidence.contains("Docker-only benchmark-created agent export/readback"))
	);
	assert_eq!(
		adapter.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-letta-core-archive-export-readback")
	);
	assert_eq!(
		adapter.pointer("/run/command").and_then(Value::as_str),
		Some(
			"ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 cargo make smoke-letta-core-archive-export-readback"
		)
	);
	assert!(adapter.pointer("/execution_metadata/setup_path").and_then(Value::as_str).is_some_and(
		|setup| setup.contains("exports core block JSON plus archival search/readback JSON")
			&& setup.contains("typed artifact")
	));

	let suites = array_at(adapter, "/suites")?;
	let core_suite = find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core_suite.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("real_world_job_adapter")
	);
	assert_eq!(adapter.pointer("/capabilities/2/status").and_then(Value::as_str), Some("blocked"));

	let scenarios = array_at(adapter, "/scenarios")?;
	let attachment = find_by_field(scenarios, "/scenario_id", "core_block_attachment_readback")?;
	let scope = find_by_field(scenarios, "/scenario_id", "core_block_scope_readback")?;
	let provenance = find_by_field(scenarios, "/scenario_id", "core_block_provenance_readback")?;
	let stale = find_by_field(scenarios, "/scenario_id", "stale_core_detection")?;
	let fallback = find_by_field(scenarios, "/scenario_id", "archival_fallback_readback")?;
	let decision =
		find_by_field(scenarios, "/scenario_id", "core_archival_project_decision_recovery")?;

	assert_eq!(scenarios.len(), 6);

	for scenario in [attachment, scope, provenance, stale, fallback, decision] {
		assert_eq!(scenario.pointer("/status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/elf_position").and_then(Value::as_str), Some("untested"));
		assert_eq!(
			scenario.pointer("/comparison_outcome").and_then(Value::as_str),
			Some("blocked")
		);
		assert_eq!(
			scenario.pointer("/command").and_then(Value::as_str),
			Some("cargo make smoke-letta-core-archive-export-readback")
		);
		assert_eq!(
			scenario.pointer("/artifact").and_then(Value::as_str),
			Some("tmp/real-world-memory/letta-core-archive/summary.json")
		);
	}

	assert_eq!(attachment.pointer("/comparison_outcome").and_then(Value::as_str), Some("blocked"));
	assert_eq!(stale.pointer("/comparison_outcome").and_then(Value::as_str), Some("blocked"));
	assert_eq!(fallback.pointer("/comparison_outcome").and_then(Value::as_str), Some("blocked"));

	Ok(())
}

fn assert_elf_fixture_adapter_record(adapter: &Value) -> Result<()> {
	assert_eq!(adapter.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert!(adapter.pointer("/run/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("60 jobs across 16 suites")
			&& evidence.contains("53 pass")
			&& evidence.contains("7 blocked")
			&& evidence.contains("core_archival_memory")
			&& evidence.contains("memory_summary")
			&& evidence.contains("proactive_brief")
			&& evidence.contains("scheduled_memory")
			&& evidence.contains("context_trajectory")
	}));

	let suites = array_at(adapter, "/suites")?;
	let core_archival = find_by_field(suites, "/suite_id", "core_archival_memory")?;
	let scheduled = find_by_field(suites, "/suite_id", "scheduled_memory")?;
	let context_trajectory = find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(core_archival.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert!(core_archival.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("core block attachment")
			&& evidence.contains("project-decision recovery")
			&& evidence.contains("archival note search")
	}));
	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(scheduled.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("4 passing source-linked task readbacks")
			&& evidence.contains("private/provider scheduler blocker")
	}));
	assert_eq!(context_trajectory.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		adapter
			.pointer("/notes/1")
			.and_then(Value::as_str)
			.is_some_and(|note| note.contains("OpenViking context-trajectory measurement gates"))
	);

	Ok(())
}

fn assert_qmd_deep_profile_gate(adapter: &Value) {
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(adapter.pointer("/run/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(adapter.pointer("/result/status").and_then(Value::as_str), Some("not_encoded"));
}

fn assert_qmd_live_baseline_record(adapter: &Value) {
	let result_evidence = adapter.pointer("/result/evidence").and_then(Value::as_str);
	let retrieval_evidence = adapter.pointer("/suites/0/evidence").and_then(Value::as_str);

	assert!(result_evidence.is_some_and(|evidence| {
		evidence.contains("This live_baseline_only record is same-corpus evidence only")
			&& evidence.contains("cite qmd_live_real_world for the full live real-world sweep")
			&& !evidence.contains("no real_world_job qmd adapter is encoded yet")
	}));
	assert!(retrieval_evidence.is_some_and(|evidence| {
		evidence.contains("does not execute real_world_job retrieval prompts")
			&& evidence.contains("cite qmd_live_real_world for the live retrieval adapter run")
			&& !evidence.contains("no real_world_job retrieval adapter run is encoded")
	}));
}

fn assert_operator_debug_live_adapter_records(elf: &Value, qmd: &Value) -> Result<()> {
	assert_eq!(elf.pointer("/evidence_class").and_then(Value::as_str), Some("live_real_world"));
	assert_eq!(elf.pointer("/overall_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make real-world-job-operator-ux-live-adapters")
	);
	assert_eq!(
		elf.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("operator_debugging_ux")
	);
	assert_eq!(elf.pointer("/suites/0/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/1/capability").and_then(Value::as_str),
		Some("trace_hydration_metadata")
	);
	assert_eq!(elf.pointer("/capabilities/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("replay_command_metadata")
	);
	assert_eq!(elf.pointer("/capabilities/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("candidate_drop_visibility")
	);
	assert_eq!(elf.pointer("/capabilities/3/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		elf.pointer("/capabilities/4/capability").and_then(Value::as_str),
		Some("openmemory_or_claude_mem_ui_runner")
	);
	assert_eq!(elf.pointer("/capabilities/4/status").and_then(Value::as_str), Some("not_encoded"));

	let elf_scenarios = array_at(elf, "/scenarios")?;
	let elf_trace = find_by_field(elf_scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let elf_replay = find_by_field(elf_scenarios, "/scenario_id", "operator_debug_replay_command")?;
	let elf_candidate =
		find_by_field(elf_scenarios, "/scenario_id", "operator_debug_candidate_drop_visibility")?;
	let elf_repair =
		find_by_field(elf_scenarios, "/scenario_id", "operator_debug_repair_action_clarity")?;
	let elf_selected =
		find_by_field(elf_scenarios, "/scenario_id", "operator_debug_selected_but_not_narrated")?;

	assert_eq!(elf_scenarios.len(), 5);
	assert_eq!(elf_trace.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(elf_trace.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(elf_replay.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(elf_candidate.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(elf_repair.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(elf_selected.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(qmd.pointer("/evidence_class").and_then(Value::as_str), Some("live_real_world"));
	assert_eq!(qmd.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("operator_debugging_ux")
	);
	assert_eq!(qmd.pointer("/suites/0/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/capabilities/1/capability").and_then(Value::as_str),
		Some("local_replay_command_metadata")
	);
	assert_eq!(qmd.pointer("/capabilities/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		qmd.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("trace_hydration_metadata")
	);
	assert_eq!(qmd.pointer("/capabilities/2/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("candidate_drop_visibility")
	);
	assert_eq!(qmd.pointer("/capabilities/3/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd.pointer("/capabilities/4/status").and_then(Value::as_str), Some("not_encoded"));

	let qmd_scenarios = array_at(qmd, "/scenarios")?;
	let qmd_trace = find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let qmd_replay = find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_replay_command")?;
	let qmd_candidate =
		find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_candidate_drop_visibility")?;
	let qmd_repair =
		find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_repair_action_clarity")?;
	let qmd_selected =
		find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_selected_but_not_narrated")?;

	assert_eq!(qmd_scenarios.len(), 5);
	assert_eq!(qmd_trace.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd_trace.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(qmd_replay.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(qmd_replay.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(qmd_candidate.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd_candidate.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(qmd_repair.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(qmd_repair.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(qmd_selected.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(qmd_selected.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert!(array_at(elf, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("narrow operator-debug live slice"))
	}));
	assert!(array_at(qmd, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("narrow operator-debug live slice"))
	}));

	Ok(())
}

fn assert_openviking_deep_profile_gate(adapter: &Value) {
	let trajectory_evidence = adapter.pointer("/capabilities/1/evidence").and_then(Value::as_str);

	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert!(trajectory_evidence.is_some_and(|evidence| {
		evidence.contains("evidence-bearing same-corpus output")
			&& evidence.contains("selected hierarchy/expansion artifacts")
			&& !evidence.contains("setup reaches runnable OpenViking APIs")
	}));
}

fn assert_first_generation_adapter_records(
	agentmemory: &Value,
	mem0: &Value,
	memsearch: &Value,
	claude_mem: &Value,
) {
	assert_agentmemory_first_generation_records(agentmemory);
	assert_mem0_first_generation_records(mem0);
	assert_memsearch_first_generation_records(memsearch);
	assert_claude_mem_first_generation_records(claude_mem);
}

fn assert_agentmemory_first_generation_records(agentmemory: &Value) {
	assert_eq!(
		agentmemory.pointer("/scenarios/1/status").and_then(Value::as_str),
		Some("lifecycle_fail")
	);
	assert_eq!(
		agentmemory.pointer("/scenarios/1/elf_position").and_then(Value::as_str),
		Some("wins")
	);
	assert_eq!(agentmemory.pointer("/scenarios/2/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		agentmemory.pointer("/scenarios/2/comparison_outcome").and_then(Value::as_str),
		Some("blocked")
	);
}

fn assert_mem0_first_generation_records(mem0: &Value) {
	assert_eq!(
		mem0.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("local_lifecycle_update_delete_reload")
	);
	assert_eq!(mem0.pointer("/capabilities/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("preference_correction_history")
	);
	assert_eq!(mem0.pointer("/capabilities/3/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/capabilities/7/capability").and_then(Value::as_str),
		Some("openmemory_ui_readback")
	);
	assert_eq!(mem0.pointer("/capabilities/7/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		mem0.pointer("/capabilities/8/capability").and_then(Value::as_str),
		Some("hosted_managed_memory_claims")
	);
	assert_eq!(mem0.pointer("/capabilities/8/status").and_then(Value::as_str), Some("unsupported"));
	assert_eq!(mem0.pointer("/scenarios/0/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0.pointer("/scenarios/0/elf_position").and_then(Value::as_str), Some("ties"));
	assert_eq!(
		mem0.pointer("/scenarios/1/scenario_id").and_then(Value::as_str),
		Some("preference_correction_history")
	);
	assert_eq!(mem0.pointer("/scenarios/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/scenarios/1/comparison_outcome").and_then(Value::as_str),
		Some("loss")
	);
	assert_eq!(
		mem0.pointer("/scenarios/5/scenario_id").and_then(Value::as_str),
		Some("openmemory_ui_export_readback")
	);
	assert_eq!(mem0.pointer("/scenarios/5/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		mem0.pointer("/scenarios/5/command").and_then(Value::as_str),
		Some("cargo make openmemory-ui-export-readback")
	);
	assert_eq!(
		mem0.pointer("/scenarios/5/artifact").and_then(Value::as_str),
		Some("tmp/live-baseline/mem0-openmemory-ui-export.json")
	);
	assert!(
		mem0.pointer("/capabilities/7/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("export-helper setup probe")
				&& evidence.contains("requires Docker access"))
	);
	assert_eq!(
		mem0.pointer("/scenarios/6/comparison_outcome").and_then(Value::as_str),
		Some("non_goal")
	);
}

fn assert_memsearch_first_generation_records(memsearch: &Value) {
	assert_eq!(
		memsearch.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("reindex_update_delete_reload")
	);
	assert_eq!(memsearch.pointer("/capabilities/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memsearch.pointer("/scenarios/0/scenario_id").and_then(Value::as_str),
		Some("canonical_markdown_reindex_reload")
	);
	assert_eq!(
		memsearch.pointer("/scenarios/0/elf_position").and_then(Value::as_str),
		Some("untested")
	);
	assert_eq!(memsearch.pointer("/suites/0/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(memsearch.pointer("/suites/0/evidence").and_then(Value::as_str).is_some_and(
		|evidence| evidence.contains("fixture-backed source-of-truth prompt coverage")
			&& evidence.contains("No live memsearch runtime adapter executes prompt scoring yet")
			&& evidence.contains("not a suite pass")
	));
	assert_eq!(memsearch.pointer("/suites/1/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(memsearch.pointer("/suites/1/evidence").and_then(Value::as_str).is_some_and(
		|evidence| evidence.contains("fixture-backed retrieval-debug prompt coverage")
			&& evidence.contains(
				"No live memsearch runtime adapter executes retrieval prompt scoring yet"
			) && evidence.contains("not a suite pass")
	));
	assert_eq!(memsearch.pointer("/scenarios/1/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memsearch.pointer("/scenarios/1/elf_position").and_then(Value::as_str),
		Some("untested")
	);
	assert_eq!(
		memsearch.pointer("/scenarios/3/status").and_then(Value::as_str),
		Some("unsupported")
	);
	assert_eq!(
		memsearch.pointer("/capabilities/4/capability").and_then(Value::as_str),
		Some("markdown_source_store_prompt_jobs")
	);
	assert_eq!(memsearch.pointer("/capabilities/4/status").and_then(Value::as_str), Some("pass"));
}

fn assert_claude_mem_first_generation_records(claude_mem: &Value) {
	assert_eq!(claude_mem.pointer("/capabilities/1/status").and_then(Value::as_str), Some("real"));
	assert_eq!(
		claude_mem.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("repository_progressive_disclosure")
	);
	assert_eq!(claude_mem.pointer("/capabilities/4/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		claude_mem.pointer("/capabilities/6/status").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(claude_mem.pointer("/suites/0/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(claude_mem.pointer("/suites/1/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		claude_mem
			.pointer("/suites/1/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("fixture-backed progressive-disclosure")
				&& evidence.contains("viewer/operator workflow remains blocked"))
	);
	assert_eq!(claude_mem.pointer("/suites/2/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		claude_mem
			.pointer("/suites/2/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("hook capture remains blocked"))
	);
	assert_eq!(
		claude_mem.pointer("/scenarios/0/status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		claude_mem.pointer("/scenarios/1/scenario_id").and_then(Value::as_str),
		Some("retrieval_repair_artifact_path")
	);
	assert_eq!(
		claude_mem.pointer("/scenarios/1/status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert!(
		claude_mem
			.pointer("/scenarios/1/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("rerun/inspection targets")
				&& evidence.contains("tmp/live-baseline/claude-mem-checks.json"))
	);
	assert_eq!(claude_mem.pointer("/scenarios/2/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(claude_mem.pointer("/scenarios/4/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(claude_mem.pointer("/scenarios/5/status").and_then(Value::as_str), Some("blocked"));
}

fn assert_graphiti_zep_adapter(adapter: &Value) {
	assert_eq!(adapter.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-graphiti-zep-docker-temporal")
	);
	assert_eq!(
		adapter.pointer("/run/command").and_then(Value::as_str),
		Some(
			"ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make smoke-graphiti-zep-docker-temporal"
		)
	);
	assert_eq!(
		adapter.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("memory_evolution")
	);
	assert_eq!(adapter.pointer("/suites/0/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/execution_metadata/research_depth").and_then(Value::as_str),
		Some(
			"D2 feasibility plus XY-888 Docker temporal smoke implementation and XY-900 scored smoke promotion; checked-in record remains research_gate unless a generated artifact reaches Graphiti search output"
		)
	);
}

fn assert_graphify_adapter(adapter: &Value) -> Result<()> {
	assert_eq!(adapter.pointer("/evidence_class").and_then(Value::as_str), Some("live_real_world"));
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(adapter.pointer("/setup/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(adapter.pointer("/run/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(adapter.pointer("/result/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		adapter.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-graphify-docker-graph-report")
	);
	assert_eq!(
		adapter.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("knowledge_compilation")
	);
	assert_eq!(adapter.pointer("/suites/0/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(adapter.pointer("/suites/1/suite_id").and_then(Value::as_str), Some("retrieval"));
	assert_eq!(adapter.pointer("/suites/1/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/execution_metadata/research_depth").and_then(Value::as_str),
		Some(
			"D1 feasibility verdict plus XY-889 Docker graph/report smoke implementation and XY-900 scored smoke promotion; current Docker validation reaches graphify output and scores the tiny knowledge_compilation job as wrong_result"
		)
	);

	let capabilities = array_at(adapter, "/capabilities")?;
	let quality = find_by_field(capabilities, "/capability", "quality_or_scale_claim")?;

	assert_eq!(quality.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(array_at(adapter, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("tiny smoke") && text.contains("non-pass"))
	}));

	Ok(())
}

fn assert_graph_rag_representative_scenarios(
	ragflow: &Value,
	lightrag: &Value,
	graphrag: &Value,
	graphiti_zep: &Value,
	graphify: &Value,
) -> Result<()> {
	let ragflow_scenarios = array_at(ragflow, "/scenarios")?;
	let lightrag_scenarios = array_at(lightrag, "/scenarios")?;
	let graphrag_scenarios = array_at(graphrag, "/scenarios")?;
	let graphiti_scenarios = array_at(graphiti_zep, "/scenarios")?;
	let graphify_scenarios = array_at(graphify, "/scenarios")?;
	let ragflow_chunk =
		find_by_field(ragflow_scenarios, "/scenario_id", "reference_chunk_citation_mapping")?;
	let lightrag_context =
		find_by_field(lightrag_scenarios, "/scenario_id", "context_source_reference_mapping")?;
	let graphrag_tables =
		find_by_field(graphrag_scenarios, "/scenario_id", "output_table_citation_mapping")?;
	let graphiti_temporal =
		find_by_field(graphiti_scenarios, "/scenario_id", "temporal_validity_window_mapping")?;
	let graphify_lint =
		find_by_field(graphify_scenarios, "/scenario_id", "graph_report_navigation_lint")?;

	assert_eq!(
		ragflow_chunk.pointer("/comparison_outcome").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(lightrag_context.pointer("/status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(
		lightrag_context.pointer("/comparison_outcome").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		graphrag_tables.pointer("/artifact").and_then(Value::as_str),
		Some(
			"apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/graphrag_output_tables_blocked.json"
		)
	);
	assert_eq!(
		graphiti_temporal.pointer("/comparison_outcome").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(graphify_lint.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		graphify_lint.pointer("/comparison_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert!(
		graphify_lint
			.pointer("/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("not an ELF victory claim"))
	);

	Ok(())
}

#[test]
fn graphify_generated_manifest_keeps_retrieval_unscored() -> Result<()> {
	let manifest = serde_json::json!({
		"schema": "elf.real_world_external_adapter_manifest/v1",
		"manifest_id": "graphify-generated-manifest-test",
		"docker_isolation": {
			"default": true,
			"compose_file": "docker-compose.baseline.yml",
			"runner": "scripts/graphify-docker-graph-report-smoke.py",
			"artifact_dir": "tmp/real-world-memory/graphify-smoke",
			"host_global_installs_required": false,
			"notes": ["Synthetic graphify generated-manifest regression test."]
		},
		"adapters": [{
			"adapter_id": "graphify_docker_smoke",
			"project": "graphify",
			"adapter_kind": "docker_cli_graph_report_smoke",
			"evidence_class": "live_real_world",
			"docker_default": true,
			"host_global_installs_required": false,
			"overall_status": "wrong_result",
			"setup": {
				"status": "pass",
				"evidence": "setup evidence",
				"command": "cargo make smoke-graphify-docker-graph-report",
				"artifact": "tmp/real-world-memory/graphify-smoke/graphify-smoke.json"
			},
			"run": {
				"status": "pass",
				"evidence": "run evidence",
				"command": "cargo make smoke-graphify-docker-graph-report",
				"artifact": "tmp/real-world-memory/graphify-smoke/summary.json"
			},
			"result": {
				"status": "wrong_result",
				"evidence": "result evidence",
				"artifact": "tmp/real-world-memory/graphify-smoke/graphify-report.json"
			},
			"capabilities": [{
				"capability": "quality_or_scale_claim",
				"status": "not_encoded",
				"evidence": "No broad graph quality claim."
			}],
			"suites": [
				{
					"suite_id": "knowledge_compilation",
					"status": "wrong_result",
					"evidence": "Only the generated graph/report evidence-mapping job is represented."
				},
				{
					"suite_id": "retrieval",
					"status": "blocked",
					"evidence": "The smoke uses graphify query output only to support source mapping; broad retrieval quality is not scored."
				}
			],
			"evidence": [],
			"execution_metadata": {
				"setup_path": "cargo make smoke-graphify-docker-graph-report",
				"runtime_boundary": "Docker-only generated graph/report smoke.",
				"resource_expectation": "Tiny generated corpus only.",
				"retry_guidance": [],
				"sources": [{
					"label": "graphify",
					"url": "https://github.com/safishamsi/graphify",
					"evidence": "Synthetic generated-manifest regression source."
				}],
				"research_depth": "Generated smoke manifest path"
			},
			"notes": ["tiny smoke non-pass"]
		}]
	});
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-graphify-manifest-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let manifest_path = temp_dir.join("manifest.json");
	let report_path = temp_dir.join("report.json");

	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixture_dir())
		.arg("--out")
		.arg(&report_path)
		.arg("--external-adapter-manifest")
		.arg(&manifest_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let report: Value = serde_json::from_slice(&fs::read(&report_path)?)?;
	let adapters = array_at(&report, "/external_adapters/adapters")?;
	let graphify = find_by_field(adapters, "/adapter_id", "graphify_docker_smoke")?;
	let suites = array_at(graphify, "/suites")?;
	let retrieval = find_by_field(suites, "/suite_id", "retrieval")?;

	assert_eq!(retrieval.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		retrieval
			.pointer("/evidence")
			.and_then(Value::as_str)
			.is_some_and(|text| { text.contains("broad retrieval quality is not scored") })
	);

	Ok(())
}

#[test]
fn graph_rag_representative_fixtures_report_typed_non_pass_states() -> Result<()> {
	let report = run_json_report_from(graph_rag_external_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(
		report.pointer("/summary/knowledge/citation_coverage").and_then(Value::as_f64),
		Some(0.667)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = array_at(&report, "/jobs")?;
	let ragflow = find_by_field(jobs, "/job_id", "graph-rag-ragflow-reference-chunks-001")?;
	let lightrag = find_by_field(jobs, "/job_id", "graph-rag-lightrag-context-sources-001")?;
	let graphrag = find_by_field(jobs, "/job_id", "graph-rag-graphrag-output-tables-001")?;
	let graphiti = find_by_field(jobs, "/job_id", "graph-rag-graphiti-temporal-validity-001")?;
	let graphify = find_by_field(jobs, "/job_id", "graph-rag-graphify-graph-report-001")?;

	assert_eq!(ragflow.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(lightrag.pointer("/status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(graphrag.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphiti.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphify.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		graphify.pointer("/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(
		graphify.pointer("/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		graphiti.pointer("/evolution/temporal_validity_not_encoded").and_then(Value::as_bool),
		Some(true)
	);
	assert!(array_contains_str(graphify, "/produced_evidence", "graphify-source-location-output")?);

	Ok(())
}

#[test]
fn live_adapter_aggregate_forwards_graph_rag_smoke_controls() -> Result<()> {
	let workspace = workspace_root()?;
	let makefile = fs::read_to_string(workspace.join("Makefile.toml"))?;
	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;

	assert!(
		makefile.contains("[tasks.real-world-memory-live-adapters]")
			&& makefile.contains("scripts/real-world-docker.sh")
			&& makefile.contains("memory-live-adapters"),
		"Makefile should expose the live-adapter command and delegate Docker details to a script",
	);

	for env_name in [
		"ELF_REAL_WORLD_LIVE_ENABLE_RAGFLOW",
		"ELF_REAL_WORLD_LIVE_ENABLE_LIGHTRAG",
		"ELF_REAL_WORLD_LIVE_ENABLE_GRAPHRAG",
		"ELF_REAL_WORLD_LIVE_ENABLE_GRAPHITI_ZEP",
		"ELF_REAL_WORLD_LIVE_ENABLE_GRAPHIFY",
		"ELF_RAGFLOW_SMOKE_START",
		"ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE",
		"ELF_GRAPHRAG_SMOKE_RUN",
		"ELF_GRAPHRAG_API_KEY",
		"ELF_GRAPHITI_ZEP_SMOKE_START",
		"ELF_GRAPHITI_ZEP_SMOKE_RUN",
		"ELF_GRAPHITI_ZEP_API_KEY",
		"ELF_GRAPHIFY_SMOKE_RUN",
	] {
		assert!(
			docker_script.contains(&format!("-e {env_name}")),
			"real-world-memory-live-adapters must forward {env_name}",
		);
	}

	assert!(
		docker_script.contains("--profile lightrag up -d lightrag"),
		"aggregate task should start LightRAG profile when ELF_LIGHTRAG_CONTEXT_START=1",
	);
	assert!(
		docker_script.contains("--profile graphiti-zep up -d graphiti-falkordb"),
		"aggregate task should start Graphiti/Zep profile when ELF_GRAPHITI_ZEP_SMOKE_START=1",
	);

	Ok(())
}

#[test]
fn openmemory_ui_export_probe_has_dedicated_docker_task() -> Result<()> {
	let workspace_root = workspace_root()?;
	let makefile = fs::read_to_string(workspace_root.join("Makefile.toml"))?;
	let docker_script = fs::read_to_string(workspace_root.join("scripts/baseline-docker.sh"))?;
	let compose = fs::read_to_string(workspace_root.join("docker-compose.baseline.yml"))?;
	let script = fs::read_to_string(workspace_root.join("scripts/live-baseline-benchmark.sh"))?;
	let report = serde_json::from_str::<Value>(&fs::read_to_string(workspace_root.join(
		"apps/elf-eval/fixtures/report_snapshots/2026-06-11-xy-931-openmemory-ui-export-readback.json",
	))?)?;

	assert!(makefile.contains("[tasks.openmemory-ui-export-readback]"));
	assert!(makefile.contains("scripts/baseline-docker.sh"));
	assert!(makefile.contains("openmemory-ui-export-readback"));
	assert!(docker_script.contains("export ELF_BASELINE_PROJECTS=mem0"));
	assert!(compose.contains("ELF_MEM0_OPENMEMORY_EXPORT_USER_ID"));
	assert!(compose.contains("ELF_MEM0_OPENMEMORY_EXPORT_CONTAINER"));
	assert!(script.contains("probe_mem0_openmemory_ui_export"));
	assert!(script.contains("mem0-openmemory-ui-export.json"));
	assert!(script.contains("DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER"));
	assert!(script.contains("sdk_get_all_is_ui_export_evidence: false"));
	assert!(
		script.contains("SDK same-corpus retrieval and every encoded SDK behavior check passed")
	);
	assert_eq!(report.pointer("/classification/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		report.pointer("/classification/reason_code").and_then(Value::as_str),
		Some("DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER")
	);
	assert_eq!(
		report
			.pointer("/same_corpus_boundary/sdk_get_all_is_ui_export_evidence")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report
			.pointer("/claim_boundary/elf_can_compare_against_openmemory_ui_export_after_this_run")
			.and_then(Value::as_bool),
		Some(false)
	);

	Ok(())
}

#[test]
fn operator_debug_live_adapter_task_is_docker_scoped() -> Result<()> {
	let workspace = workspace_root()?;
	let makefile = fs::read_to_string(workspace.join("Makefile.toml"))?;
	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;
	let script = fs::read_to_string(
		workspace.join("scripts").join("real-world-operator-debug-live-adapters.sh"),
	)?;
	let live_adapter =
		fs::read_to_string(workspace.join("apps/elf-eval/src/bin/real_world_live_adapter.rs"))?;
	let benchmark =
		fs::read_to_string(workspace.join("apps/elf-eval/src/bin/real_world_job_benchmark.rs"))?;

	assert!(makefile.contains("[tasks.real-world-job-operator-ux-live-adapters]"));
	assert!(makefile.contains("scripts/real-world-docker.sh"));
	assert!(makefile.contains("job-operator-ux-live-adapters"));
	assert!(
		docker_script.contains("docker compose -f docker-compose.baseline.yml run --build --rm")
	);
	assert!(docker_script.contains("scripts/real-world-operator-debug-live-adapters.sh"));
	assert!(script.contains("apps/elf-eval/fixtures/real_world_job/operator_debugging_ux"));
	assert!(script.contains("elf_operator_debug_live"));
	assert!(script.contains("qmd_operator_debug_live"));
	assert!(script.contains("elf.real_world_operator_debug_live_adapter_sweep/v1"));
	assert!(script.contains("trace_available"));
	assert!(script.contains("replay_command_available"));
	assert!(live_adapter.contains("fn operator_debug_output("));
	assert!(live_adapter.contains("fn qmd_replay_command("));
	assert!(live_adapter.contains("fn elf_replay_command("));
	assert!(
		!live_adapter
			.contains("does not yet hydrate full operator trace/viewer diagnostics for this suite")
	);
	assert!(benchmark.contains("Replay command:"));
	assert!(benchmark.contains("replay_command_available"));

	Ok(())
}

#[test]
fn external_adapter_manifest_rejects_unmeasured_win_loss_scenario_outcomes() -> Result<()> {
	let output = run_external_manifest_with_letta_attachment_mutation(
		"invalid-scenario-outcome-test",
		|scenario| {
			set_json_pointer(scenario, "/status", serde_json::json!("not_encoded"))?;

			set_json_pointer(scenario, "/comparison_outcome", serde_json::json!("win"))
		},
	)?;

	assert!(!output.status.success(), "invalid scenario outcome unexpectedly passed");
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("not_encoded status with win outcome")
	);

	Ok(())
}

#[test]
fn external_adapter_manifest_rejects_unmeasured_win_loss_scenario_positions() -> Result<()> {
	let output = run_external_manifest_with_letta_attachment_mutation(
		"invalid-scenario-position-test",
		|scenario| {
			set_json_pointer(scenario, "/status", serde_json::json!("not_encoded"))?;
			set_json_pointer(scenario, "/elf_position", serde_json::json!("wins"))?;

			set_json_pointer(scenario, "/comparison_outcome", serde_json::json!("not_tested"))
		},
	)?;

	assert!(!output.status.success(), "invalid scenario position unexpectedly passed");
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("not_encoded status with wins position")
	);

	Ok(())
}

#[test]
fn external_adapter_manifest_rejects_blocked_status_without_blocked_outcome() -> Result<()> {
	let output = run_external_manifest_scenario_mutation(
		"invalid-blocked-scenario-outcome-test",
		"letta_research_gate",
		"stale_core_detection",
		|scenario| {
			scenario
				.as_object_mut()
				.ok_or_else(|| eyre::eyre!("scenario is not an object"))?
				.remove("comparison_outcome");

			Ok(())
		},
	)?;

	assert!(!output.status.success(), "invalid blocked scenario unexpectedly passed");
	assert!(
		String::from_utf8_lossy(&output.stderr)
			.contains("blocked status without blocked comparison outcome")
	);

	Ok(())
}

#[test]
fn external_adapter_manifest_rejects_conflicting_scenario_position_and_outcome() -> Result<()> {
	let output = run_external_manifest_with_letta_attachment_mutation(
		"invalid-scenario-position-outcome-test",
		|scenario| {
			set_json_pointer(scenario, "/status", serde_json::json!("pass"))?;
			set_json_pointer(scenario, "/elf_position", serde_json::json!("ties"))?;

			set_json_pointer(scenario, "/comparison_outcome", serde_json::json!("loss"))
		},
	)?;

	assert!(!output.status.success(), "conflicting scenario unexpectedly passed");
	assert!(String::from_utf8_lossy(&output.stderr).contains("ties position with loss outcome"));

	Ok(())
}

#[test]
fn live_adapter_supports_elf_capture_write_policy_without_external_hook_claims() -> Result<()> {
	let workspace = workspace_root()?;
	let live_adapter =
		fs::read_to_string(workspace.join("apps/elf-eval/src/bin/real_world_live_adapter.rs"))?;
	let live_script =
		fs::read_to_string(workspace.join("scripts").join("real-world-live-adapters.sh"))?;
	let manifest = fs::read_to_string(
		workspace
			.join("apps/elf-eval/fixtures/real_world_external_adapters")
			.join("memory_projects_manifest.json"),
	)?;

	assert!(live_adapter.contains("fn is_elf_capture_live_adapter("));
	assert!(live_adapter.contains("suite == \"capture_integration\""));
	assert!(live_adapter.contains("write_policy_audit_count"));
	assert!(live_adapter.contains("excluded_evidence_ids"));
	assert!(live_adapter.contains("source_id"));
	assert!(live_adapter.contains("runtime_source_refs"));
	assert!(live_adapter.contains("validate_capture_runtime_evidence"));
	assert!(live_adapter.contains("capture_failure"));
	assert!(live_adapter.contains("fn materialize_elf_consolidation("));
	assert!(live_adapter.contains("ConsolidationProposalReviewRequest"));
	assert!(live_adapter.contains("fn materialize_elf_knowledge("));
	assert!(live_adapter.contains("KnowledgePageLintRequest"));
	assert!(live_script.contains("OPERATOR_FIXTURE_DIR"));
	assert!(live_script.contains("INPUT_FIXTURE_DIR"));
	assert!(live_script.contains("operator_debugging_ux"));
	assert!(manifest.contains("\"scenario_id\": \"live_capture_write_policy\""));
	assert!(manifest.contains("\"scenario_id\": \"capture_write_policy_hooks\""));
	assert!(manifest.contains("\"comparison_outcome\": \"blocked\""));
	assert!(manifest.contains("Four redaction, exclusion, source-id, evidence-binding"));
	assert!(manifest.contains("durable upstream agentmemory session/capture path"));
	assert!(manifest.contains("Docker-contained session directory"));
	assert!(manifest.contains("claude-mem hooks, viewer, timeline, and observation workflows"));

	Ok(())
}

#[test]
fn declared_not_encoded_consolidation_jobs_do_not_require_fake_proposals() -> Result<()> {
	let fixture_path = consolidation_fixture_dir().join("contradiction_report_discard.json");
	let mut fixture = serde_json::from_str::<Value>(&fs::read_to_string(fixture_path)?)?;

	fixture
		.pointer_mut("/corpus/adapter_response")
		.and_then(Value::as_object_mut)
		.ok_or_else(|| eyre::eyre!("missing adapter_response object"))?
		.remove("consolidation");

	let encoding = serde_json::json!({
		"status": "not_encoded",
		"reason": "The qmd live adapter retrieves evidence-linked answers but does not generate or review consolidation proposals."
	});

	fixture
		.as_object_mut()
		.ok_or_else(|| eyre::eyre!("fixture is not an object"))?
		.insert("encoding".to_string(), encoding);

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-not-encoded-consolidation-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("not_encoded_consolidation.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "consolidation-contradiction-report-discard-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn capture_write_policy_live_report_preserves_competitor_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		capture_write_policy_live_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(capture_write_policy_live_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.capture_write_policy_live_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-933"));
	assert_eq!(
		report
			.pointer("/live_capture_results/elf_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report
			.pointer("/live_capture_results/elf_live_real_world/encoded_job_count")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/live_capture_results/elf_live_real_world/redaction_leak_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/live_capture_results/qmd_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("not_encoded")
	);

	let jobs = array_at(&report, "/jobs")?;
	let source_binding = find_by_field(jobs, "/job_id", "capture-source-id-binding-001")?;
	let source_binding_refs = array_at(source_binding, "/runtime_source_refs")?;
	let release_summary_ref =
		find_by_field(source_binding_refs, "/evidence_id", "source-id-release-summary")?;

	assert!(array_contains_str(source_binding, "/source_ids", "capture:issue-comment-42")?);
	assert_eq!(
		release_summary_ref.pointer("/source_id").and_then(Value::as_str),
		Some("capture:issue-comment-42")
	);
	assert_eq!(
		release_summary_ref.pointer("/evidence_binding").and_then(Value::as_str),
		Some("source_ref")
	);

	let write_policy = find_by_field(jobs, "/job_id", "capture-write-policy-redaction-001")?;

	assert_eq!(
		write_policy.pointer("/write_policy_redaction_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		write_policy
			.pointer("/runtime_source_refs/0/write_policy_applied")
			.and_then(Value::as_bool),
		Some(true)
	);

	let boundary = find_by_field(jobs, "/job_id", "capture-integration-boundaries-001")?;

	assert!(array_contains_str(boundary, "/excluded_evidence_ids", "private-span-trap")?);
	assert!(!array_contains_str(boundary, "/stored_evidence_ids", "private-span-trap")?);
	assert!(
		array_at(boundary, "/runtime_source_refs")?
			.iter()
			.all(|item| item.pointer("/evidence_id").and_then(Value::as_str)
				!= Some("private-span-trap"))
	);

	let positions = array_at(&report, "/competitor_positions")?;
	let qmd = find_by_field(positions, "/project", "qmd")?;
	let agentmemory = find_by_field(positions, "/project", "agentmemory")?;
	let claude_mem = find_by_field(positions, "/project", "claude-mem")?;

	assert_eq!(qmd.pointer("/position").and_then(Value::as_str), Some("untested"));
	assert!(qmd.pointer("/reason").and_then(Value::as_str).is_some_and(|reason| {
		reason.contains("typed not_encoded") && reason.contains("ELF self-check")
	}));
	assert_eq!(agentmemory.pointer("/position").and_then(Value::as_str), Some("blocked"));
	assert!(agentmemory.pointer("/reason").and_then(Value::as_str).is_some_and(|reason| {
		reason.contains("process-local StateKV Map") && reason.contains("in-memory index")
	}));
	assert_eq!(claude_mem.pointer("/position").and_then(Value::as_str), Some("blocked"));
	assert!(
		claude_mem
			.pointer("/reason")
			.and_then(Value::as_str)
			.is_some_and(|reason| reason.contains("hooks, timeline, observations")
				&& reason.contains("Docker-contained hook/viewer runner"))
	);
	assert!(markdown.contains("ELF now has live capture/write-policy self-check evidence"));
	assert!(markdown.contains("not an ELF-over-qmd win"));
	assert!(markdown.contains("| claude-mem capture/viewer flows | `blocked` |"));
	assert!(!markdown.contains("claude-mem capture breadth is untested"));
	assert!(markdown.contains("runtime `source_ref` metadata returned by search"));
	assert!(markdown.contains("Do not claim ELF broadly beats agentmemory or claude-mem"));
	assert!(benchmarking_index.contains("2026-06-11-capture-write-policy-live-report.md"));
	assert!(readme.contains("Capture/Write-Policy Live Report - June 11, 2026"));
	assert!(readme.contains("mem0/OpenMemory"));
	assert!(readme.contains("and memsearch now pass their scoped local baseline"));
	assert!(
		collapse_whitespace(&readme)
			.contains("claude-mem hook/viewer capture remains blocked until Docker-contained")
	);

	Ok(())
}

#[test]
fn live_consolidation_report_preserves_reviewable_output_boundaries() -> Result<()> {
	let workspace = workspace_root()?;
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		live_consolidation_proposal_scoring_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(live_consolidation_proposal_scoring_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;
	let benchmark_runbook = fs::read_to_string(
		workspace
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("real_world_agent_memory_benchmark.md"),
	)?;
	let makefile = fs::read_to_string(workspace.join("Makefile.toml"))?;
	let live_script =
		fs::read_to_string(workspace.join("scripts/real-world-consolidation-live-adapter.sh"))?;
	let live_adapter =
		fs::read_to_string(workspace.join("apps/elf-eval/src/bin/real_world_live_adapter.rs"))?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.live_consolidation_proposal_scoring_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-934"));
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/encoded_job_count")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/proposal_count")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/source_mutation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/review_event_count")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/qmd_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("not_encoded")
	);

	let jobs = array_at(&report, "/jobs")?;
	let project_summary =
		find_by_field(jobs, "/job_id", "consolidation-project-summary-apply-001")?;
	let preference =
		find_by_field(jobs, "/job_id", "consolidation-preference-candidate-defer-001")?;
	let contradiction =
		find_by_field(jobs, "/job_id", "consolidation-contradiction-report-discard-001")?;

	assert_eq!(
		project_summary.pointer("/final_review_state").and_then(Value::as_str),
		Some("applied")
	);
	assert_eq!(project_summary.pointer("/review_event_count").and_then(Value::as_u64), Some(2));
	assert_eq!(preference.pointer("/final_review_state").and_then(Value::as_str), Some("archived"));
	assert_eq!(
		contradiction.pointer("/final_review_state").and_then(Value::as_str),
		Some("rejected")
	);
	assert_eq!(
		contradiction.pointer("/unsupported_claim_flag_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(contradiction.pointer("/source_lineage_count").and_then(Value::as_u64), Some(3));

	let positions = array_at(&report, "/reference_positions")?;
	let qmd = find_by_field(positions, "/project", "qmd")?;
	let managed = find_by_field(positions, "/project", "managed_dreaming_memory_systems")?;
	let always_on = find_by_field(positions, "/project", "always_on_memory_agent_patterns")?;

	assert_eq!(qmd.pointer("/position").and_then(Value::as_str), Some("untested"));
	assert_eq!(managed.pointer("/position").and_then(Value::as_str), Some("product_reference"));
	assert_eq!(always_on.pointer("/position").and_then(Value::as_str), Some("product_reference"));
	assert!(markdown.contains("ELF now has service-backed live consolidation proposal scoring"));
	assert!(markdown.contains("This is not scheduled production consolidation"));
	assert!(markdown.contains("Source mutations"));
	assert!(markdown.contains("Do not mix knowledge-page rebuild/lint scoring"));
	assert!(
		benchmarking_index.contains("2026-06-16-live-consolidation-proposal-scoring-report.md")
	);
	assert!(readme.contains("Live Consolidation Proposal Scoring Report - June 16, 2026"));
	assert!(readme.contains("real-world-memory-live-consolidation"));
	assert!(benchmark_runbook.contains("Current live consolidation increment"));
	assert!(benchmark_runbook.contains("tmp/real-world-memory/live-consolidation/summary.json"));
	assert!(makefile.contains("[tasks.real-world-memory-live-consolidation]"));
	assert!(makefile.contains("scripts/real-world-docker.sh"));

	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;

	assert!(docker_script.contains("scripts/real-world-consolidation-live-adapter.sh"));
	assert!(live_script.contains("elf.real_world_consolidation_live_adapter_sweep/v1"));
	assert!(live_script.contains("real_world_live_adapter -- elf"));
	assert!(!live_script.contains("real_world_live_adapter -- qmd"));
	assert!(live_adapter.contains("fn materialize_elf_consolidation("));
	assert!(live_adapter.contains("ConsolidationProposalReviewRequest"));

	Ok(())
}

#[test]
fn live_knowledge_page_rebuild_lint_has_dedicated_docker_task() -> Result<()> {
	let workspace = workspace_root()?;
	let makefile = fs::read_to_string(workspace.join("Makefile.toml"))?;
	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;
	let live_script =
		fs::read_to_string(workspace.join("scripts/real-world-knowledge-live-adapter.sh"))?;
	let live_adapter =
		fs::read_to_string(workspace.join("apps/elf-eval/src/bin/real_world_live_adapter.rs"))?;
	let benchmark_runbook = fs::read_to_string(
		workspace
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("real_world_agent_memory_benchmark.md"),
	)?;
	let live_runbook = fs::read_to_string(
		workspace
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("live_baseline_benchmark.md"),
	)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert!(makefile.contains("[tasks.real-world-memory-live-knowledge]"));
	assert!(makefile.contains("scripts/real-world-docker.sh"));
	assert!(makefile.contains("memory-live-knowledge"));
	assert!(docker_script.contains("memory-live-knowledge)"));
	assert!(docker_script.contains("-e ELF_KNOWLEDGE_LIVE_REPORT_DIR"));
	assert!(docker_script.contains("-e ELF_KNOWLEDGE_LIVE_FIXTURES"));
	assert!(docker_script.contains("scripts/real-world-knowledge-live-adapter.sh"));
	assert!(live_script.contains("elf.real_world_knowledge_live_adapter_sweep/v1"));
	assert!(live_script.contains("apps/elf-eval/fixtures/real_world_memory/knowledge"));
	assert!(live_script.contains("tmp/real-world-memory/live-knowledge"));
	assert!(live_script.contains("real-world-memory-live-knowledge"));
	assert!(live_script.contains("ElfService knowledge_page_rebuild"));
	assert!(live_script.contains("knowledge_page_lint"));
	assert!(live_script.contains("knowledge_pages_search"));
	assert!(live_script.contains("pages remain derived benchmark artifacts"));
	assert!(live_adapter.contains("fn materialize_elf_knowledge("));
	assert!(live_adapter.contains("KnowledgePageRebuildRequest"));
	assert!(live_adapter.contains("KnowledgePageLintRequest"));
	assert!(live_adapter.contains("KnowledgePageSearchRequest"));
	assert!(benchmark_runbook.contains("Current live knowledge-page rebuild/lint increment"));
	assert!(benchmark_runbook.contains("cargo make real-world-memory-live-knowledge"));
	assert!(benchmark_runbook.contains("tmp/real-world-memory/live-knowledge/summary.json"));
	assert!(live_runbook.contains("cargo make real-world-memory-live-knowledge"));
	assert!(benchmarking_index.contains("2026-06-20-live-knowledge-page-rebuild-lint-report.md"));
	assert!(readme.contains("Live Knowledge-Page Rebuild/Lint Report - June 20, 2026"));

	Ok(())
}

fn assert_live_sweep_record(adapter: &Value, production_ops_status: &str) -> Result<()> {
	let suites = array_at(adapter, "/suites")?;
	let capabilities = array_at(adapter, "/capabilities")?;
	let adapter_id = adapter.pointer("/adapter_id").and_then(Value::as_str).unwrap_or_default();
	let targeted = find_by_field(capabilities, "/capability", "targeted_live_pass")?;
	let full_pass = find_by_field(capabilities, "/capability", "full_suite_live_pass")?;
	let work_resume = find_by_field(suites, "/suite_id", "work_resume")?;
	let memory_evolution = find_by_field(suites, "/suite_id", "memory_evolution")?;
	let production_ops = find_by_field(suites, "/suite_id", "production_ops")?;
	let consolidation = find_by_field(suites, "/suite_id", "consolidation")?;
	let knowledge = find_by_field(suites, "/suite_id", "knowledge_compilation")?;
	let operator_debug = find_by_field(suites, "/suite_id", "operator_debugging_ux")?;
	let capture = find_by_field(suites, "/suite_id", "capture_integration")?;
	let personalization = find_by_field(suites, "/suite_id", "personalization")?;
	let core_archival = find_by_field(suites, "/suite_id", "core_archival_memory")?;
	let context_trajectory = find_by_field(suites, "/suite_id", "context_trajectory")?;
	let trust_sot = find_by_field(suites, "/suite_id", "trust_source_of_truth")?;
	let retrieval = find_by_field(suites, "/suite_id", "retrieval")?;
	let project_decisions = find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(suites.len(), 13);
	assert_eq!(targeted.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(full_pass.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert!(
		adapter
			.pointer("/result/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("55 jobs across all 13 checked-in suites"))
	);
	assert_eq!(trust_sot.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(work_resume.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(retrieval.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(project_decisions.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		production_ops.pointer("/status").and_then(Value::as_str),
		Some(production_ops_status)
	);

	if adapter_id == "elf_live_real_world" {
		assert_eq!(consolidation.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert_eq!(knowledge.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert_eq!(operator_debug.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("pass"));
		assert!(
			capture
				.pointer("/evidence")
				.and_then(Value::as_str)
				.is_some_and(|evidence| evidence.contains("4/4 capture_integration jobs"))
		);
	} else {
		assert_eq!(consolidation.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
		assert_eq!(knowledge.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
		assert_eq!(operator_debug.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
		assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	}

	assert_eq!(personalization.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_archival.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(context_trajectory.pointer("/status").and_then(Value::as_str), Some("blocked"));

	Ok(())
}

#[test]
fn runner_discovers_nested_fixture_layout() -> Result<()> {
	let report = run_json_report_from(fixture_root())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(62));

	Ok(())
}

#[test]
fn operator_debug_fixture_reports_trace_links_and_failure_details() -> Result<()> {
	let report = run_json_report_from(operator_debug_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(
		report.pointer("/summary/operator_debug_job_count").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(report.pointer("/summary/raw_sql_needed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/trace_incomplete_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/operator_ux_gap_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(2)
	);

	let jobs = array_at(&report, "/jobs")?;
	let dropped = find_by_field(jobs, "/job_id", "operator-debug-dropped-evidence-001")?;
	let selected = find_by_field(jobs, "/job_id", "operator-debug-selected-not-narrated-001")?;

	assert_eq!(dropped.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		dropped.pointer("/operator_debug/raw_sql_needed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		dropped.pointer("/operator_debug/dropped_candidate_visibility").and_then(Value::as_str),
		Some("visible in Retrieval Funnel and Replay Candidates")
	);
	assert_eq!(
		dropped.pointer("/operator_debug/viewer_url").and_then(Value::as_str),
		Some("/viewer?trace_id=11111111-1111-4111-8111-111111111111")
	);
	assert_eq!(
		dropped.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("filter.read_profile")
	);
	assert!(array_contains_str(
		dropped,
		"/trace_explainability/stages/1/dropped_evidence",
		"trace-dropped-expected"
	)?);
	assert!(array_contains_str(
		dropped,
		"/trace_explainability/stages/1/distractor_evidence",
		"trace-dropped-decoy"
	)?);
	assert!(array_contains_str(dropped, "/produced_evidence", "trace-dropped-expected")?);
	assert_eq!(selected.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		selected.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("selection.narration")
	);
	assert_eq!(
		selected.pointer("/operator_debug/failure_mode").and_then(Value::as_str),
		Some("selected_but_not_narrated")
	);

	Ok(())
}

#[test]
fn consolidation_fixtures_report_reviewable_proposal_metrics() -> Result<()> {
	let report = run_json_report_from(consolidation_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(
		report.pointer("/summary/consolidation/proposal_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/consolidation/proposal_unsupported_claim_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/executable_gap_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/lineage_completeness").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/review_action_correctness").and_then(Value::as_f64),
		Some(1.0)
	);

	let jobs = array_at(&report, "/jobs")?;
	let project_summary =
		find_by_field(jobs, "/job_id", "consolidation-project-summary-apply-001")?;
	let contradiction =
		find_by_field(jobs, "/job_id", "consolidation-contradiction-report-discard-001")?;

	assert_eq!(
		project_summary
			.pointer("/consolidation/proposals/0/actual_review_action")
			.and_then(Value::as_str),
		Some("apply")
	);
	assert_eq!(
		contradiction
			.pointer("/consolidation/proposals/0/actual_review_action")
			.and_then(Value::as_str),
		Some("discard")
	);
	assert_eq!(
		contradiction
			.pointer("/consolidation/proposals/0/unsupported_claim_count")
			.and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(&report, "/suites")?;
	let consolidation_suite = find_by_field(suites, "/suite_id", "consolidation")?;

	assert_eq!(consolidation_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	Ok(())
}

#[test]
fn knowledge_fixtures_report_page_metrics() -> Result<()> {
	let report = run_json_report_from(knowledge_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(4));
	assert_eq!(
		report.pointer("/summary/knowledge/section_count").and_then(Value::as_u64),
		Some(10)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/citation_coverage").and_then(Value::as_f64),
		Some(0.9)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/rebuild_determinism").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_count").and_then(Value::as_u64),
		Some(9)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/pages_with_backlinks").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.969)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/allowed_variance_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(&report, "/suites")?;
	let knowledge_suite = find_by_field(suites, "/suite_id", "knowledge_compilation")?;

	assert_eq!(knowledge_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(knowledge_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(2));

	let jobs = array_at(&report, "/jobs")?;
	let project_page_job = find_by_field(jobs, "/job_id", "knowledge-project-page-001")?;

	assert_eq!(
		project_page_job.pointer("/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		project_page_job.pointer("/knowledge/untraced_section_count").and_then(Value::as_u64),
		Some(0)
	);

	Ok(())
}

#[test]
fn project_decisions_fixtures_report_decision_policy_cases() -> Result<()> {
	let report = run_json_report_from(project_decisions_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);

	let suites = array_at(&report, "/suites")?;
	let project_decisions = find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(project_decisions.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(project_decisions.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		project_decisions.pointer("/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);

	let jobs = array_at(&report, "/jobs")?;
	let accepted = find_by_field(jobs, "/job_id", "project-decision-accepted-typed-failures-001")?;
	let reversal = find_by_field(jobs, "/job_id", "project-decision-reversal-live-baseline-001")?;
	let validation =
		find_by_field(jobs, "/job_id", "project-decision-current-validation-gate-001")?;
	let tradeoff = find_by_field(jobs, "/job_id", "project-decision-tradeoff-fixture-backed-001")?;
	let caveat = find_by_field(jobs, "/job_id", "project-decision-private-manifest-caveat-001")?;

	assert_eq!(accepted.pointer("/answer_type").and_then(Value::as_str), Some("decision_record"));
	assert_eq!(
		accepted.pointer("/expected_evidence").and_then(Value::as_array).map(Vec::len),
		Some(2)
	);
	assert_eq!(
		reversal.pointer("/evolution/historical_evidence/0").and_then(Value::as_str),
		Some("live-baseline-suite-win-old")
	);
	assert_eq!(
		validation.pointer("/evolution/current_evidence/0").and_then(Value::as_str),
		Some("validation-gate-current-decodex")
	);
	assert_eq!(tradeoff.pointer("/requires_caveat").and_then(Value::as_bool), Some(true));
	assert_eq!(caveat.pointer("/can_answer_unknown").and_then(Value::as_bool), Some(true));

	for job in jobs {
		let expected_evidence = array_at(job, "/expected_evidence")?;

		assert!(
			!expected_evidence.is_empty(),
			"project decision job {} must declare required evidence",
			job.pointer("/job_id").and_then(Value::as_str).unwrap_or("<unknown>")
		);
	}
	for entry in fs::read_dir(project_decisions_fixture_dir())? {
		let path = entry?.path();

		if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
			continue;
		}

		let fixture = serde_json::from_str::<Value>(&fs::read_to_string(path)?)?;
		let required_evidence = array_at(&fixture, "/required_evidence")?;
		let negative_traps = array_at(&fixture, "/negative_traps")?;

		assert!(!required_evidence.is_empty());
		assert!(!negative_traps.is_empty());
	}

	Ok(())
}

#[test]
fn qmd_openviking_strength_profile_report_preserves_claim_boundaries() -> Result<()> {
	let report =
		serde_json::from_str::<Value>(&fs::read_to_string(strength_profile_report_path()?)?)?;
	let markdown = fs::read_to_string(strength_profile_markdown_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let iteration_direction = fs::read_to_string(iteration_direction_report_path()?)?;

	assert_strength_profile_summary(&report);
	assert_strength_profile_terms(&report)?;
	assert_qmd_strength_profile(&report)?;
	assert_qmd_wrong_result_diagnosis(&report)?;
	assert_openviking_strength_profile(&report)?;
	assert_strength_profile_json_claim_boundaries(&report)?;
	assert_strength_profile_markdown_boundaries(&markdown);
	assert_operator_facing_strength_profile_boundaries(
		&readme,
		&benchmarking_index,
		&iteration_direction,
	);

	Ok(())
}

#[test]
fn current_benchmark_reports_preserve_live_sweep_boundaries() -> Result<()> {
	let measurement_audit = fs::read_to_string(measurement_coverage_audit_path()?)?;
	let measurement_audit_json = serde_json::from_str::<Value>(&fs::read_to_string(
		measurement_coverage_audit_json_path()?,
	)?)?;
	let competitor_matrix = fs::read_to_string(competitor_strength_matrix_path()?)?;
	let competitor_matrix_json = serde_json::from_str::<Value>(&fs::read_to_string(
		competitor_strength_matrix_json_path()?,
	)?)?;
	let iteration_direction = fs::read_to_string(iteration_direction_report_path()?)?;
	let external_manifest = fs::read_to_string(external_adapter_manifest_path())?;
	let comparison_external_projects = fs::read_to_string(comparison_external_projects_path()?)?;
	let retrieval_debug_profile =
		serde_json::from_str::<Value>(&fs::read_to_string(retrieval_debug_profile_json_path()?)?)?;
	let temporal_history = serde_json::from_str::<Value>(&fs::read_to_string(
		temporal_history_competitor_gap_json_path()?,
	)?)?;

	assert_current_report_text_boundaries(
		&measurement_audit,
		&competitor_matrix,
		&iteration_direction,
		&external_manifest,
		&comparison_external_projects,
	);

	assert!(competitor_matrix.contains("claude-mem work_resume remains `not_encoded`"));
	assert!(!competitor_matrix.contains("claude-mem `wrong_result`, OpenViking work_resume"));

	let qmd_live = find_by_field(
		array_at(&measurement_audit_json, "/live_real_world_adapters")?,
		"/adapter",
		"qmd live CLI adapter",
	)?;

	assert_eq!(qmd_live.pointer("/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(qmd_live.pointer("/wrong_result").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd_live.pointer("/expected_evidence_matched").and_then(Value::as_u64), Some(38));
	assert_eq!(qmd_live.pointer("/evidence_covered_count").and_then(Value::as_u64), Some(45));

	let memory_evolution = find_by_field(
		array_at(&measurement_audit_json, "/live_suite_breakdown")?,
		"/suite",
		"memory_evolution",
	)?;

	assert_eq!(
		memory_evolution.pointer("/elf_status_counts/wrong_result").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		memory_evolution.pointer("/qmd_status_counts/wrong_result").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		retrieval_debug_profile
			.pointer("/live_real_world_full_sweep_context/qmd/pass")
			.and_then(Value::as_u64),
		Some(17)
	);
	assert_eq!(
		retrieval_debug_profile
			.pointer("/live_real_world_full_sweep_context/qmd/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);

	assert_competitor_strength_matrix_json(&competitor_matrix_json)?;

	let openmemory_command = find_by_field(
		array_at(&temporal_history, "/commands")?,
		"/command",
		"cargo make openmemory-ui-export-readback",
	)?;

	assert!(
		openmemory_command
			.pointer("/artifact")
			.and_then(Value::as_str)
			.is_some_and(|artifact| artifact.contains("tmp/live-baseline/mem0-checks.json")
				&& artifact.contains("tmp/live-baseline/mem0-openmemory-ui-export.json"))
	);

	Ok(())
}

fn assert_current_report_text_boundaries(
	measurement_audit: &str,
	competitor_matrix: &str,
	iteration_direction: &str,
	external_manifest: &str,
	comparison_external_projects: &str,
) {
	assert!(
		measurement_audit.contains(
			"| `memory_evolution` | `6` | `pass:1`, `wrong_result:5` | `wrong_result:6` |"
		)
	);
	assert!(
		measurement_audit
			.contains("qmd live fails 6/6 jobs after missing the delete/TTL tombstone evidence")
	);
	assert!(measurement_audit.contains("Basic local smoke and local OSS history/readback pass"));
	assert!(measurement_audit.contains("claude-mem hook/viewer capture is `blocked`"));
	assert!(!measurement_audit.contains("claude-mem hook/viewer capture remains untested"));
	assert!(!measurement_audit.contains("blocked or untested"));

	assert_measurement_audit_adapter_status_counts(measurement_audit);

	assert!(
		competitor_matrix
			.contains("broader live suites remain `wrong_result`, `blocked`, or `not_encoded`")
	);
	assert!(competitor_matrix.contains(
		"Overall adapter-status counts: 4 `pass`,\n6 `wrong_result`, 1 `lifecycle_fail`, 7 `blocked`, and 5 `not_encoded`."
	));
	assert!(!competitor_matrix.contains("5 `blocked`, and 7 `not_encoded`"));
	assert!(
		competitor_matrix
			.contains("mem0/OpenMemory local OSS entity-scoped personalization now passes")
	);
	assert!(competitor_matrix.contains("scoped preference behavior is a measured tie"));
	assert!(
		!competitor_matrix.contains("mem0/OpenMemory and Letta personalization are `not_encoded`")
	);
	assert!(external_manifest.contains(
		"The record is a full-suite sweep, not a full-suite pass; wrong_result, blocked, and not_encoded states remain visible."
	));
	assert!(external_manifest.contains(
		"The qmd live real-world sweep covers the current encoded fixture corpus; expanded retrieval-debug strength suites still need their own materialized adapter run."
	));
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for scoped local OSS same-corpus retrieval")
	);
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for local same-corpus retrieval, reindex/update/delete")
	);
	assert!(iteration_direction.contains("| Jobs | `55` |"));
	assert!(iteration_direction.contains("| Encoded suites | `15` |"));
	assert!(iteration_direction.contains("| Pass | `49` |"));
	assert!(iteration_direction.contains("| Evidence coverage | `123/123` |"));
	assert!(iteration_direction.contains("| Expected evidence recall | `115/115` |"));

	for stale_phrase in [
		"same live sweep shape as ELF",
		"ELF and qmd live fail 5/6 jobs",
		"both systems currently fail 5/6 live memory-evolution jobs",
		"wrong_result, incomplete, blocked, and not_encoded states remain visible",
		"broader live suites remain `wrong_result`, `incomplete`, or `not_encoded`",
		"The qmd live real-world slice covers representative jobs only",
		"| Jobs | `40` |",
		"| Encoded suites | `11` |",
		"| Jobs | `50` |",
		"| Encoded suites | `14` |",
		"| Pass | `38` |",
		"| Pass | `45` |",
		"| Evidence coverage | `115/115` |",
		"| Expected evidence recall | `107/107` |",
		"history/UI/hosted/graph behavior remains",
		"current local adapter is incomplete/wrong-result",
		"current adapter is incomplete/invalid-result",
	] {
		assert!(!measurement_audit.contains(stale_phrase));
		assert!(!competitor_matrix.contains(stale_phrase));
		assert!(!iteration_direction.contains(stale_phrase));
		assert!(!external_manifest.contains(stale_phrase));
		assert!(!comparison_external_projects.contains(stale_phrase));
	}
}

#[test]
fn live_temporal_reconciliation_report_records_xy905_before_after() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		live_temporal_reconciliation_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(live_temporal_reconciliation_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.live_temporal_reconciliation_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-905"));
	assert_eq!(
		report
			.pointer("/baseline/elf_memory_evolution/job_status_counts/pass")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/baseline/elf_memory_evolution/job_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/post_stage/elf_memory_evolution/job_status_counts/pass")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/post_stage/elf_memory_evolution/job_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/post_stage/elf_memory_evolution/suite_status").and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report.pointer("/post_stage/qmd_memory_evolution/suite_status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		report
			.pointer("/comparison_judgment/current_vs_historical_correctness")
			.and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(
		report
			.pointer("/comparison_judgment/deletion_ttl_tombstone_behavior")
			.and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(array_contains_str(
		&report,
		"/trace_contract/answer_fields",
		"selected_historical_evidence"
	)?);
	assert!(array_contains_str(
		&report,
		"/trace_contract/materialization_fields",
		"current_winner_evidence_ids"
	)?);
	assert!(array_contains_str(
		&report,
		"/trace_contract/trace_stages",
		"temporal_reconciliation.conflict_candidates"
	)?);
	assert!(report.pointer("/trace_contract/negative_gate").and_then(Value::as_str).is_some_and(
		|gate| gate.contains("selected conflict evidence id") && gate.contains("wrong_result")
	));
	assert!(markdown.contains("ELF passing all six memory-evolution jobs"));
	assert!(markdown.contains("selected-but-not-narrated conflicts as `wrong_result`"));
	assert!(markdown.contains("Do not claim ELF beats Graphiti/Zep"));
	assert!(benchmarking_index.contains("2026-06-16-live-temporal-reconciliation-report.md"));
	assert!(
		readme.contains("Live Temporal Reconciliation Report - June 16, 2026")
			&& readme.contains("now reports ELF live `memory_evolution` as 6/6 pass")
	);

	Ok(())
}

#[test]
fn dreaming_competitor_strength_retest_report_closes_xy955_without_overclaims() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		dreaming_competitor_strength_retest_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(dreaming_competitor_strength_retest_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_competitor_strength_retest_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-955"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("locally_and_partially_stronger_only")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/regressed_stage_count").and_then(Value::as_u64), Some(0));
	assert!(array_contains_str(&report, "/status_terms", "typed_non_pass")?);
	assert!(array_contains_str(
		&report,
		"/summary/unsupported_claims_rejected",
		"ELF does not broadly beat qmd from this retest."
	)?);

	assert_xy955_commands(&report)?;
	assert_xy955_stage_closeout(&report)?;
	assert_xy955_scenario_retests(&report)?;
	assert_xy955_optimization_queue(&report)?;
	assert_xy955_follow_up_issue_briefs(&report)?;

	assert!(markdown.contains("ELF is locally and partially stronger"));
	assert!(
		markdown.contains("The full live-adapter command now has fresh ELF and qmd scored reports")
	);
	assert!(
		markdown.contains(
			"Do not treat qmd full-suite wrong_result counts as a regression of qmd debug"
		)
	);
	assert!(markdown.contains("## Follow-Up Issue Briefs"));
	assert!(markdown.contains(
		"| GraphRAG/LightRAG/RAGFlow/llm-wiki/gbrain/graphify citation/navigation/knowledge surfaces |"
	));
	assert!(
		benchmarking_index.contains("2026-06-17-dreaming-competitor-strength-retest-report.md")
	);
	assert!(readme.contains("Dreaming Competitor-Strength Retest Report - June 17, 2026"));
	assert!(readme.contains("17 competitor-strength closeout"));

	Ok(())
}

#[test]
fn qmd_debug_ergonomics_dreaming_retest_report_preserves_qmd_edge() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		qmd_debug_ergonomics_dreaming_retest_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(qmd_debug_ergonomics_dreaming_retest_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_qmd_debug_retest_summary(&report)?;
	assert_qmd_debug_retest_command_and_adapters(&report)?;
	assert_qmd_debug_retest_scenarios(&report)?;
	assert_qmd_debug_retest_boundaries(&report)?;
	assert_qmd_debug_retest_markdown_and_indexes(&markdown, &benchmarking_index, &readme);

	Ok(())
}

fn assert_qmd_debug_retest_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.qmd_debug_ergonomics_dreaming_retest_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-982"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("unchanged_with_live_operator_debug_confirmation")
	);
	assert_eq!(
		report.pointer("/summary/debug_ergonomics_edge").and_then(Value::as_str),
		Some("qmd_default_top10_and_short_cli_replay_preserved")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/improved_scenario_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/regressed_scenario_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/unchanged_scenario_count").and_then(Value::as_u64),
		Some(6)
	);
	assert!(array_contains_str(
		report,
		"/summary/unsupported_claims_rejected",
		"qmd's live operator-debug wrong_result rows do not erase qmd's default top-k and short CLI replay edge."
	)?);

	Ok(())
}

fn assert_qmd_debug_retest_command_and_adapters(report: &Value) -> Result<()> {
	let command = find_by_field(
		array_at(report, "/commands")?,
		"/command",
		"cargo make real-world-job-operator-ux-live-adapters",
	)?;

	assert_eq!(command.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		command.pointer("/summary/schema").and_then(Value::as_str),
		Some("elf.real_world_operator_debug_live_adapter_sweep/v1")
	);

	let adapters = array_at(report, "/adapter_summaries")?;
	let elf = find_by_field(adapters, "/adapter_id", "elf_operator_debug_live")?;
	let qmd = find_by_field(adapters, "/adapter_id", "qmd_operator_debug_live")?;

	assert_eq!(elf.pointer("/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(elf.pointer("/trace_available_count").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/replay_command_available_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(qmd.pointer("/wrong_result").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/trace_available_count").and_then(Value::as_u64), Some(0));
	assert_eq!(qmd.pointer("/trace_incomplete_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/replay_command_available_count").and_then(Value::as_u64), Some(6));

	Ok(())
}

fn assert_qmd_debug_retest_scenarios(report: &Value) -> Result<()> {
	let scenarios = array_at(report, "/scenario_retests")?;
	let top10 = find_by_field(scenarios, "/scenario_id", "qmd_default_top10_candidate_artifact")?;
	let replay = find_by_field(scenarios, "/scenario_id", "qmd_short_cli_replay")?;
	let trace = find_by_field(scenarios, "/scenario_id", "elf_operator_debug_trace_hydration")?;
	let candidate =
		find_by_field(scenarios, "/scenario_id", "operator_debug_candidate_drop_visibility")?;
	let expansion = find_by_field(scenarios, "/scenario_id", "query_expansion_attribution")?;
	let fusion = find_by_field(scenarios, "/scenario_id", "fusion_attribution")?;
	let rerank = find_by_field(scenarios, "/scenario_id", "rerank_attribution")?;

	assert_eq!(scenarios.len(), 10);
	assert_eq!(top10.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(top10.pointer("/current_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(replay.pointer("/current_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(
		trace.pointer("/current_counts/elf_trace_available").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		trace.pointer("/current_counts/qmd_trace_available").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		candidate
			.pointer("/current_counts/qmd_intermediate_stage_visible_jobs")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert!(array_contains_str(candidate, "/typed_non_pass_states", "retrieved_but_dropped")?);
	assert_eq!(expansion.pointer("/judgment").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(fusion.pointer("/judgment").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(rerank.pointer("/judgment").and_then(Value::as_str), Some("non_goal"));

	Ok(())
}

fn assert_qmd_debug_retest_boundaries(report: &Value) -> Result<()> {
	assert!(array_contains_str(
		report,
		"/claim_boundaries/allowed",
		"qmd's default local-debug edge remains: top-10 candidate rows plus short CLI replay."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF broadly beats qmd from this retest."
	)?);
	assert!(array_contains_str(
		report,
		"/next_optimization_direction/required_fields",
		"fusion_rank_deltas"
	)?);

	Ok(())
}

fn assert_qmd_debug_retest_markdown_and_indexes(
	markdown: &str,
	benchmarking_index: &str,
	readme: &str,
) {
	assert!(markdown.contains("The qmd debug-ergonomics outcome is unchanged"));
	assert!(markdown.contains("ELF 6 pass/0 wrong_result; qmd 0 pass/6 wrong_result"));
	assert!(
		markdown.contains("Do not treat qmd's 0 pass/6 wrong_result live operator-debug slice")
	);
	assert!(markdown.contains("Immediate top-k rows with source id"));
	assert!(
		benchmarking_index.contains("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md")
	);
	assert!(readme.contains("qmd Debug-Ergonomics Dreaming Retest Report - June 19, 2026"));
	assert!(readme.contains("Latest real-world benchmark report: June 20, 2026"));
	assert!(readme.contains("keeps the qmd edge unchanged"));
}

#[test]
fn openviking_trajectory_materialization_report_preserves_blocked_gates() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		openviking_trajectory_materialization_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(openviking_trajectory_materialization_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_openviking_trajectory_materialization_summary(&report)?;
	assert_openviking_trajectory_materialization_command(&report)?;
	assert_openviking_trajectory_materialization_scenarios(&report)?;
	assert_openviking_trajectory_materialization_boundaries(&report)?;
	assert_openviking_trajectory_materialization_markdown_and_indexes(
		&markdown,
		&benchmarking_index,
		&readme,
	);

	Ok(())
}

#[test]
fn letta_core_archive_export_readback_report_preserves_blocked_gates() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		letta_core_archive_export_readback_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(letta_core_archive_export_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.letta_core_archive_export_readback_summary/v1")
	);
	assert_eq!(
		report.pointer("/adapter_id").and_then(Value::as_str),
		Some("letta_core_archive_export_readback")
	);
	assert_eq!(
		report.pointer("/materialization/status/failure_class").and_then(Value::as_str),
		Some("letta_live_run_disabled")
	);
	assert_eq!(
		report.pointer("/materialization/status/overall").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/status").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/counts/blocked").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/counts/pass").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/scored_benchmark/counts/wrong_result")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/scored_benchmark/evidence_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/materialization/benchmark_input/core_blocks")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(9)
	);
	assert_eq!(
		report
			.pointer("/materialization/benchmark_input/archival_passages")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/materialization/evidence_mapping/expected_evidence_ids")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(14)
	);
	assert_eq!(
		report
			.pointer("/materialization/evidence_mapping/mapped_evidence_ids")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/improvement_regression_readback/judgment")
			.and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(array_contains_str(
		&report,
		"/materialization/claim_boundaries/not_allowed",
		"Do not claim ELF beats Letta on core-vs-archival memory from fixture-only ELF evidence."
	)?);
	assert!(markdown.contains("The Letta follow-up is now reproducible"));
	assert!(markdown.contains("6 typed blocked"));
	assert!(markdown.contains("competitive status is unchanged"));
	assert!(benchmarking_index.contains("2026-06-19-letta-core-archive-export-readback-report.md"));
	assert!(readme.contains("Letta core/archive materialization after XY-984"));
	assert!(readme.contains("smoke-letta-core-archive-export-readback"));

	Ok(())
}

#[test]
fn service_native_dreaming_readback_report_materializes_public_jobs() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		service_native_dreaming_readback_report_json_path()?,
	)?)?;
	let materialization = serde_json::from_str::<Value>(&fs::read_to_string(
		service_native_dreaming_readback_materialization_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(service_native_dreaming_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_service_native_dreaming_report_summary(&report)?;
	assert_service_native_dreaming_report_jobs(&report)?;
	assert_service_native_dreaming_materialization(&materialization)?;
	assert_service_native_dreaming_docs(&markdown, &benchmarking_index, &readme);

	Ok(())
}

fn assert_service_native_dreaming_report_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/adapter/adapter_id").and_then(Value::as_str),
		Some("elf_service_native_dreaming")
	);
	assert_eq!(
		report.pointer("/adapter/behavior").and_then(Value::as_str),
		Some("service_native_dreaming_readback")
	);
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(11));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(9));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	let suites = array_at(report, "/suites")?;
	let memory = find_by_field(suites, "/suite_id", "memory_summary")?;
	let proactive = find_by_field(suites, "/suite_id", "proactive_brief")?;
	let scheduled = find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(memory.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));

	Ok(())
}

fn assert_service_native_dreaming_report_jobs(report: &Value) -> Result<()> {
	let jobs = array_at(report, "/jobs")?;
	let memory = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;
	let daily = find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private_brief =
		find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;
	let weekly = find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;
	let private_scheduled =
		find_by_field(jobs, "/job_id", "scheduled-private-provider-scheduler-blocked-001")?;

	assert_eq!(memory.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(daily.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(private_brief.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(private_scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(!array_contains_str(memory, "/produced_evidence", "stale-summary-gap")?);
	assert!(!array_contains_str(memory, "/produced_evidence", "summary-temporary-claim")?);
	assert!(!array_contains_str(daily, "/produced_evidence", "daily-old-parity-trap")?);
	assert!(!array_contains_str(
		weekly,
		"/produced_evidence",
		"scheduled-weekly-hosted-parity-trap"
	)?);

	Ok(())
}

fn assert_service_native_dreaming_materialization(materialization: &Value) -> Result<()> {
	assert_eq!(
		materialization.pointer("/schema").and_then(Value::as_str),
		Some("elf.real_world_live_adapter_materialization/v1")
	);
	assert_eq!(
		materialization.pointer("/adapter_id").and_then(Value::as_str),
		Some("elf_service_native_dreaming")
	);
	assert_eq!(materialization.pointer("/status").and_then(Value::as_str), Some("blocked"));

	let jobs = array_at(materialization, "/jobs")?;
	let memory = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;
	let daily = find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private_brief =
		find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;

	for job in jobs {
		match job.pointer("/status").and_then(Value::as_str) {
			Some("pass") => {
				assert_eq!(
					job.pointer("/dreaming_readback/runtime_path").and_then(Value::as_str),
					Some("ElfService::add_note -> ElfService::list -> derived readback artifact")
				);
				assert!(array_at(job, "/dreaming_readback/missing_source_refs")?.is_empty());
				assert_eq!(
					job.pointer("/dreaming_readback/source_mutation_count").and_then(Value::as_u64),
					Some(0)
				);
				assert_eq!(
					job.pointer("/dreaming_readback/no_source_mutation_checked")
						.and_then(Value::as_bool),
					Some(true)
				);
			},
			Some("blocked") => {
				assert!(job.pointer("/dreaming_readback").is_none_or(Value::is_null));
			},
			status => {
				return Err(eyre::eyre!(
					"unexpected service-native materialization status: {status:?}"
				));
			},
		}
	}

	assert!(array_contains_str(
		memory,
		"/dreaming_readback/selected_source_refs",
		"stale-summary-gap"
	)?);
	assert!(!array_contains_str(memory, "/evidence_ids", "stale-summary-gap")?);
	assert!(array_contains_str(
		daily,
		"/dreaming_readback/selected_source_refs",
		"daily-old-parity-trap"
	)?);
	assert!(!array_contains_str(daily, "/evidence_ids", "daily-old-parity-trap")?);
	assert!(private_brief.pointer("/dreaming_readback").is_none_or(Value::is_null));

	Ok(())
}

fn assert_service_native_dreaming_docs(markdown: &str, benchmarking_index: &str, readme: &str) {
	assert!(markdown.contains("9 pass"));
	assert!(markdown.contains("0 wrong_result"));
	assert!(markdown.contains("2 typed blocked"));
	assert!(markdown.contains("ElfService::add_note -> ElfService::list"));
	assert!(markdown.contains("Do not claim ELF broadly beats OpenAI Pulse"));
	assert!(benchmarking_index.contains("2026-06-19-service-native-dreaming-readback-report.md"));
	assert!(readme.contains("Service-native Dreaming readback after XY-986"));
	assert!(readme.contains("real-world-memory-service-native-dreaming"));
}

#[test]
fn operator_approved_public_proxy_private_addendum_preserves_boundary() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		operator_approved_public_proxy_private_addendum_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(
		operator_approved_public_proxy_private_addendum_report_markdown_path()?,
	)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.operator_approved_public_proxy_baseline_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-930"));
	assert_eq!(report.pointer("/command/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		report.pointer("/command/run_id").and_then(Value::as_str),
		Some("live-baseline-20260619143959")
	);
	assert_eq!(
		report.pointer("/corpus/profile").and_then(Value::as_str),
		Some("production-private")
	);
	assert_eq!(
		report.pointer("/corpus/runner_track").and_then(Value::as_str),
		Some("private_production")
	);
	assert_eq!(
		report.pointer("/corpus/manifest_kind").and_then(Value::as_str),
		Some("operator_approved_public_proxy")
	);
	assert_eq!(
		report.pointer("/corpus/manifest_id").and_then(Value::as_str),
		Some("operator-approved-public-proxy-prod-corpus-2026-06-19")
	);
	assert_eq!(report.pointer("/embedding/mode").and_then(Value::as_str), Some("local"));
	assert_eq!(
		report.pointer("/embedding/provider_backed_quality_proven").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(report.pointer("/summary/project_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/check_summary/total").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/check_summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(
		report.pointer("/query_summary/wrong_result_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/backfill/completed_count").and_then(Value::as_u64), Some(12));
	assert_eq!(report.pointer("/backfill/duplicate_source_notes").and_then(Value::as_u64), Some(0));

	let queries = array_at(&report, "/queries")?;
	let provider = find_by_field(queries, "/id", "q-explain-provider-blocker")?;

	assert_eq!(queries.len(), 8);
	assert_eq!(
		provider.pointer("/top_evidence").and_then(Value::as_str),
		Some("blocker-provider-missing")
	);
	assert_eq!(provider.pointer("/matched").and_then(Value::as_bool), Some(true));
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not call this real private-corpus production proof."
	)?);
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim provider-backed production quality; embedding mode was local."
	)?);
	assert!(array_contains_str(
		&report,
		"/improvement_regression_readback/unchanged",
		"Real private-corpus production quality is still not proven."
	)?);
	assert!(array_contains_str(
		&report,
		"/next_optimization_direction/when_operator_inputs_exist",
		"Run provider-backed embeddings with ELF_BASELINE_ELF_EMBEDDING_MODE=provider and a routed provider setup."
	)?);
	assert!(markdown.contains("proxy corpus pass"));
	assert!(markdown.contains("Do not call this real private-corpus production proof."));
	assert!(markdown.contains("| Embedding mode | `local` |"));
	assert!(
		benchmarking_index
			.contains("2026-06-19-operator-approved-public-proxy-production-private-addendum.md")
	);
	assert!(benchmarking_index.contains("not real private-corpus or provider-backed proof"));
	assert!(readme.contains("Operator-approved public-proxy addendum after XY-930"));
	assert!(readme.contains("8/8 query passes"));
	assert!(readme.contains("does not prove real private-corpus production quality"));

	Ok(())
}

#[test]
fn openmemory_ui_export_product_recheck_preserves_blocked_boundary() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		openmemory_ui_export_product_readback_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(openmemory_ui_export_product_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.openmemory_ui_export_product_recheck_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-987"));
	assert_eq!(
		report.pointer("/command/command").and_then(Value::as_str),
		Some("cargo make openmemory-ui-export-readback")
	);
	assert_eq!(report.pointer("/command/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		report.pointer("/command/probe_artifact").and_then(Value::as_str),
		Some("tmp/live-baseline/mem0-openmemory-ui-export.json")
	);
	assert_eq!(report.pointer("/run/sdk_check_summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/run/ui_export_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		report.pointer("/run/ui_export_reason_code").and_then(Value::as_str),
		Some("DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER")
	);
	assert_eq!(
		report
			.pointer("/same_corpus_boundary/sdk_get_all_is_ui_export_evidence")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report
			.pointer("/openmemory_product_surface/export_requires_running_container")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert!(
		report
			.pointer("/openmemory_probe/attempt/output_excerpt")
			.and_then(Value::as_str)
			.is_some_and(|excerpt| excerpt.contains("docker: command not found")
				&& excerpt.contains("Container 'openmemory-openmemory-mcp-1' not found/running"))
	);
	assert_eq!(
		report.pointer("/classification/comparison_judgment").and_then(Value::as_str),
		Some("unchanged")
	);
	assert_eq!(
		report
			.pointer("/claim_boundary/product_browser_or_dashboard_readback_reached")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert!(array_contains_str(
		&report,
		"/improvement_regression_readback/unchanged",
		"OpenMemory product UI/export readback remains blocked before same-corpus product app database validation."
	)?);
	assert!(array_contains_str(
		&report,
		"/next_optimization_direction/required_fields",
		"same_corpus_import_into_openmemory_app_database"
	)?);
	assert!(markdown.contains("OpenMemory UI/export product-readback status is unchanged"));
	assert!(markdown.contains("Product browser/dashboard readback reached"));
	assert!(
		benchmarking_index.contains("2026-06-19-openmemory-ui-export-product-readback-report.md")
	);
	assert!(readme.contains("OpenMemory UI/Export Product Readback Report - June 19, 2026"));
	assert!(readme.contains("OpenMemory UI/export product recheck after XY-987"));

	Ok(())
}

#[test]
fn graph_rag_citation_navigation_promotion_preserves_typed_non_passes() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		graph_rag_citation_navigation_promotion_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(graph_rag_citation_navigation_promotion_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.graph_rag_citation_navigation_promotion_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-985"));
	assert_eq!(
		report.pointer("/command/command").and_then(Value::as_str),
		Some("cargo make real-world-memory-graph-rag")
	);
	assert_eq!(report.pointer("/command/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("unchanged_typed_non_pass")
	);
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(0.25));
	assert_eq!(
		report.pointer("/summary/knowledge_citation_coverage").and_then(Value::as_f64),
		Some(0.667)
	);

	let scenarios = array_at(&report, "/scenario_outcomes")?;
	let ragflow = find_by_field(scenarios, "/project", "RAGFlow")?;
	let lightrag = find_by_field(scenarios, "/project", "LightRAG")?;
	let graphrag = find_by_field(scenarios, "/project", "GraphRAG")?;
	let graphiti = find_by_field(scenarios, "/project", "Graphiti/Zep")?;
	let graphify = find_by_field(scenarios, "/project", "graphify")?;
	let llm_wiki = find_by_field(scenarios, "/project", "llm-wiki")?;
	let gbrain = find_by_field(scenarios, "/project", "gbrain")?;

	assert_eq!(ragflow.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(lightrag.pointer("/current_status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(graphrag.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphiti.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphify.pointer("/current_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(llm_wiki.pointer("/current_status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(gbrain.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert!(array_contains_str(graphify, "/produced_evidence", "graphify-source-location-output")?);
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim graph/RAG parity or broad graph-navigation quality."
	)?);
	assert!(array_contains_str(
		&report,
		"/next_optimization_direction/required_fields",
		"graphrag_output_table_rows_with_generated_evidence_ids"
	)?);
	assert!(markdown.contains("typed non-pass, no parity claim"));
	assert!(
		markdown.contains("graphify produces evidence-linked output but still scores wrong_result")
	);
	assert!(
		benchmarking_index.contains("2026-06-19-graph-rag-citation-navigation-promotion-report.md")
	);
	assert!(readme.contains("Graph/RAG Citation and Navigation Promotion Report - June 19, 2026"));
	assert!(readme.contains("Graph/RAG citation/navigation promotion after XY-985"));

	Ok(())
}

fn assert_openviking_trajectory_materialization_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.openviking_trajectory_materialization_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-983"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("materialized_blocked_context_trajectory_evidence")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/blockers_removed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked_scenario_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/regressed_scenario_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert!(array_contains_str(
		report,
		"/summary/unsupported_claims_rejected",
		"ELF does not beat OpenViking staged retrieval trajectory from fixture-only blocked rows."
	)?);

	Ok(())
}

fn assert_openviking_trajectory_materialization_command(report: &Value) -> Result<()> {
	let command = find_by_field(
		array_at(report, "/commands")?,
		"/command",
		"cargo make real-world-memory-context-trajectory",
	)?;
	let summary =
		command.pointer("/summary").ok_or_else(|| eyre::eyre!("missing command summary"))?;

	assert_eq!(command.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		command.pointer("/artifact_json").and_then(Value::as_str),
		Some("tmp/real-world-memory/context-trajectory/report.json")
	);
	assert_eq!(summary.pointer("/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(summary.pointer("/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(summary.pointer("/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(summary.pointer("/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(summary.pointer("/evidence_covered_count").and_then(Value::as_u64), Some(9));
	assert_eq!(summary.pointer("/source_ref_covered_count").and_then(Value::as_u64), Some(9));
	assert_eq!(summary.pointer("/quote_covered_count").and_then(Value::as_u64), Some(9));

	Ok(())
}

fn assert_openviking_trajectory_materialization_scenarios(report: &Value) -> Result<()> {
	let scenarios = array_at(report, "/scenario_materialization")?;
	let staged =
		find_by_field(scenarios, "/scenario_id", "openviking_staged_retrieval_trajectory")?;
	let hierarchy = find_by_field(scenarios, "/scenario_id", "openviking_hierarchy_selection")?;
	let recursive =
		find_by_field(scenarios, "/scenario_id", "openviking_recursive_context_expansion")?;

	assert_eq!(scenarios.len(), 3);

	for scenario in [staged, hierarchy, recursive] {
		assert_eq!(scenario.pointer("/previous_status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	}

	assert!(array_contains_str(
		staged,
		"/produced_evidence",
		"openviking-evidence-id-output-contract"
	)?);
	assert!(array_contains_str(
		hierarchy,
		"/produced_evidence",
		"hierarchy-selection-output-contract"
	)?);
	assert!(array_contains_str(
		recursive,
		"/produced_evidence",
		"recursive-expansion-output-contract"
	)?);
	assert_eq!(
		staged.pointer("/claim_boundary").and_then(Value::as_str),
		Some(
			"No ELF win, tie, or loss is allowed until both systems publish comparable stage artifacts for the same context-trajectory scenario."
		)
	);
	assert_eq!(
		hierarchy.pointer("/blocker").and_then(Value::as_str),
		Some("selected_hierarchy_nodes_and_evidence_ids_missing")
	);
	assert_eq!(
		recursive.pointer("/blocker").and_then(Value::as_str),
		Some("expansion_paths_and_same_corpus_evidence_ids_missing")
	);

	Ok(())
}

fn assert_openviking_trajectory_materialization_boundaries(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/improvement_regression_readback/improved").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/improvement_regression_readback/blocked").and_then(Value::as_u64),
		Some(3)
	);
	assert!(array_contains_str(
		report,
		"/claim_boundaries/allowed",
		"The context-trajectory slice is now reproducible through cargo make real-world-memory-context-trajectory."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF beats OpenViking on staged retrieval trajectory."
	)?);
	assert!(array_contains_str(
		report,
		"/next_optimization_direction/required_fields",
		"expansion_path"
	)?);
	assert_eq!(
		report.pointer("/next_optimization_direction/non_goal").and_then(Value::as_str),
		Some(
			"No ELF product change or superiority claim is authorized by this materialization-only report."
		)
	);

	Ok(())
}

fn assert_openviking_trajectory_materialization_markdown_and_indexes(
	markdown: &str,
	benchmarking_index: &str,
	readme: &str,
) {
	assert!(markdown.contains("The OpenViking trajectory follow-up is now materialized"));
	assert!(markdown.contains("3 encoded jobs, 0 pass, 3 blocked, 9/9 evidence coverage"));
	assert!(markdown.contains("Do not claim ELF beats OpenViking on staged retrieval trajectory."));
	assert!(markdown.contains("OpenViking context-trajectory job can move from `blocked`"));
	assert!(
		benchmarking_index.contains("2026-06-19-openviking-trajectory-materialization-report.md")
	);
	assert!(readme.contains("OpenViking Trajectory Materialization Report - June 19, 2026"));
	assert!(readme.contains("cargo make real-world-memory-context-trajectory"));
	assert!(readme.contains("3 typed blockers with 9/9 evidence coverage"));
}

fn assert_xy955_commands(report: &Value) -> Result<()> {
	let commands = array_at(report, "/commands")?;
	let aggregate = find_by_field(commands, "/command", "cargo make real-world-memory")?;
	let graph_rag = find_by_field(commands, "/command", "cargo make real-world-memory-graph-rag")?;
	let first_generation =
		find_by_field(commands, "/command", "cargo make real-world-first-generation-oss")?;
	let live = find_by_field(commands, "/command", "cargo make real-world-memory-live-adapters")?;

	assert_eq!(aggregate.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(aggregate.pointer("/summary/pass").and_then(Value::as_u64), Some(53));
	assert_eq!(aggregate.pointer("/summary/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(graph_rag.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(graph_rag.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(graph_rag.pointer("/summary/incomplete").and_then(Value::as_u64), Some(1));
	assert_eq!(graph_rag.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(first_generation.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(first_generation.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(live.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		live.pointer("/partial_summary/elf_live_real_world/pass").and_then(Value::as_u64),
		Some(40)
	);
	assert_eq!(
		live.pointer("/partial_summary/elf_live_real_world/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		live.pointer("/partial_summary/qmd_live_real_world/pass").and_then(Value::as_u64),
		Some(17)
	);
	assert_eq!(
		live.pointer("/partial_summary/qmd_live_real_world/wrong_result").and_then(Value::as_u64),
		Some(13)
	);

	Ok(())
}

fn assert_xy955_stage_closeout(report: &Value) -> Result<()> {
	let stages = array_at(report, "/stage_closeout")?;

	assert_eq!(stages.len(), 8);

	let current = find_by_field(stages, "/stage_id", "current_vs_historical_correctness")?;
	let proactive = find_by_field(stages, "/stage_id", "proactive_brief_readiness")?;
	let scheduled = find_by_field(stages, "/stage_id", "scheduled_memory_task_readiness")?;
	let final_retest = find_by_field(stages, "/stage_id", "final_competitor_retest_status")?;

	assert_eq!(current.pointer("/judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(current.pointer("/current_counts/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(current.pointer("/current_counts/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(proactive.pointer("/judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(proactive.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(scheduled.pointer("/current_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(scheduled.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(final_retest.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(final_retest.pointer("/current_counts/pass").and_then(Value::as_u64), Some(40));
	assert_eq!(
		final_retest.pointer("/current_counts/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(final_retest.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(
		final_retest.pointer("/current_counts/not_encoded").and_then(Value::as_u64),
		Some(19)
	);
	assert!(final_retest.pointer("/boundary").and_then(Value::as_str).is_some_and(|boundary| {
		boundary.contains("qmd now has a fresh scored live report")
			&& boundary.contains("broader superiority is not proven")
	}));
	assert_eq!(final_retest.pointer("/qmd_current_counts/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(
		final_retest.pointer("/qmd_current_counts/wrong_result").and_then(Value::as_u64),
		Some(13)
	);

	Ok(())
}

fn assert_xy955_scenario_retests(report: &Value) -> Result<()> {
	let scenarios = array_at(report, "/scenario_retests")?;
	let qmd = find_by_field(scenarios, "/scenario_id", "qmd_debug_ergonomics")?;
	let mem0 =
		find_by_field(scenarios, "/scenario_id", "mem0_openmemory_preference_history_export")?;
	let letta = find_by_field(scenarios, "/scenario_id", "letta_core_archive")?;
	let graph_rag = find_by_field(
		scenarios,
		"/scenario_id",
		"graph_rag_citation_navigation_knowledge_surfaces",
	)?;
	let private_provider =
		find_by_field(scenarios, "/scenario_id", "private_provider_production_gates")?;

	assert_eq!(qmd.pointer("/current_outcome").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(qmd.pointer("/current_status").and_then(Value::as_str), Some("pass"));
	assert!(qmd.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("17 pass")
			&& evidence.contains("13 wrong_result")
			&& evidence.contains("does not retest or erase")
	}));
	assert_eq!(mem0.pointer("/current_outcome").and_then(Value::as_str), Some("unchanged"));
	assert!(mem0.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("mem0/OpenMemory local OSS history")
			&& evidence.contains("OpenMemory UI/export remains setup-blocked")
	}));
	assert_eq!(letta.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		graph_rag.pointer("/current_status").and_then(Value::as_str),
		Some("typed_non_pass")
	);
	assert!(graph_rag.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("0 pass")
			&& evidence.contains("1 wrong_result")
			&& evidence.contains("3 blocked")
	}));
	assert_eq!(private_provider.pointer("/follow_up").and_then(Value::as_str), Some("XY-930"));

	Ok(())
}

fn assert_xy955_optimization_queue(report: &Value) -> Result<()> {
	let queue = array_at(report, "/optimization_queue")?;
	let qmd = find_by_field(queue, "/issue", "XY-923")?;
	let private_provider = find_by_field(queue, "/issue", "XY-930")?;
	let openviking = find_by_field(queue, "/issue", "XY-928")?;
	let letta = find_by_field(queue, "/issue", "letta-core-archive-adapter-brief")?;
	let service_native = find_by_field(queue, "/issue", "service-native-dreaming-outputs-brief")?;

	assert_eq!(qmd.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(private_provider.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(openviking.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(letta.pointer("/status").and_then(Value::as_str), Some("proposed"));
	assert_eq!(service_native.pointer("/status").and_then(Value::as_str), Some("proposed"));
	assert!(array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not treat qmd full-suite wrong_result counts as a regression of qmd debug ergonomics."
	)?);

	Ok(())
}

fn assert_xy955_follow_up_issue_briefs(report: &Value) -> Result<()> {
	let existing = array_at(report, "/follow_up_issue_briefs/existing")?;
	let proposed = array_at(report, "/follow_up_issue_briefs/proposed")?;
	let qmd = find_by_field(existing, "/issue", "XY-923")?;
	let private_provider = find_by_field(existing, "/issue", "XY-930")?;
	let letta = find_by_field(proposed, "/issue", "letta-core-archive-adapter-brief")?;
	let service_native =
		find_by_field(proposed, "/issue", "service-native-dreaming-outputs-brief")?;

	assert!(qmd.pointer("/scope").and_then(Value::as_str).is_some_and(|scope| {
		scope.contains("immediate top-k") && scope.contains("candidate-drop artifacts")
	}));
	assert!(qmd.pointer("/non_goal").and_then(Value::as_str).is_some_and(|non_goal| {
		non_goal.contains("qmd full-suite wrong_result counts")
			&& non_goal.contains("debug ergonomics")
	}));
	assert!(
		private_provider
			.pointer("/non_goal")
			.and_then(Value::as_str)
			.is_some_and(|non_goal| non_goal.contains("Do not infer credentials"))
	);
	assert!(letta.pointer("/validation").and_then(Value::as_str).is_some_and(|validation| {
		validation.contains("Letta core block JSON") && validation.contains("typed outcome states")
	}));
	assert!(
		service_native
			.pointer("/non_goal")
			.and_then(Value::as_str)
			.is_some_and(|non_goal| non_goal.contains("Pulse clone"))
	);

	Ok(())
}

#[test]
fn qmd_trace_replay_diagnostics_report_preserves_claim_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		trace_replay_diagnostics_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(trace_replay_diagnostics_markdown_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let adoption_report = fs::read_to_string(competitor_strength_adoption_report_path()?)?;
	let adoption_json = serde_json::from_str::<Value>(&fs::read_to_string(
		competitor_strength_adoption_report_json_path()?,
	)?)?;

	assert_trace_replay_diagnostics_json(&report)?;
	assert_trace_replay_diagnostics_markdown(&markdown);

	assert!(readme.contains("ELF/qmd Trace Replay Diagnostics Report - June 11, 2026"));
	assert!(benchmarking_index.contains("2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"));
	assert!(benchmarking_index.contains("qmd top-10/replay artifact"));
	assert!(benchmarking_index.contains("ELF trace/admin surfaces"));
	assert!(adoption_report.contains("| Retrieval quality and local debug UX | `loss` |"));
	assert!(adoption_report.contains("Letta scenario rows remain"));
	assert!(adoption_report.contains("blocked or `not_tested`"));

	assert_trace_replay_viewer_blocker_boundaries(
		&readme,
		&markdown,
		&adoption_report,
		&report,
		&adoption_json,
	)?;

	assert!(
		adoption_report
			.contains("Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF")
	);
	assert!(array_at(&adoption_json, "/adoption_decision/remaining_caveats")?.iter().any(
		|caveat| {
			caveat.as_str().is_some_and(|text| {
				text.contains("Letta scenario rows remain blocked or not_tested")
			})
		}
	));

	assert_trace_replay_adoption_json(&adoption_json)?;

	Ok(())
}

fn assert_trace_replay_diagnostics_json(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.trace_replay_diagnostics_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-923"));
	assert_eq!(
		string_array_at(report, "/outcome_terms")?,
		["win", "tie", "loss", "not_tested", "blocked", "non_goal"].map(str::to_owned)
	);
	assert_eq!(
		report.pointer("/summary/retrieval_correctness").and_then(Value::as_str),
		Some("tie")
	);
	assert_eq!(report.pointer("/summary/outcome_counts/loss").and_then(Value::as_u64), Some(2));
	assert_eq!(
		report.pointer("/summary/outcome_counts/not_tested").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(report.pointer("/summary/outcome_counts/win").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/outcome_counts/tie").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/outcome_counts/non_goal").and_then(Value::as_u64), Some(1));

	let scenarios = array_at(report, "/scenario_outcomes")?;
	let retrieval = find_by_field(scenarios, "/scenario_id", "retrieval_correctness_guardrail")?;
	let top10 = find_by_field(scenarios, "/scenario_id", "default_top10_candidate_artifact")?;
	let replay = find_by_field(scenarios, "/scenario_id", "replay_command_locality")?;
	let trace_surface =
		find_by_field(scenarios, "/scenario_id", "trace_admin_replay_surface_availability")?;
	let operator_trace =
		find_by_field(scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let operator_replay =
		find_by_field(scenarios, "/scenario_id", "operator_debug_replay_command_availability")?;
	let operator_candidate =
		find_by_field(scenarios, "/scenario_id", "operator_debug_candidate_drop_visibility")?;
	let operator_repair =
		find_by_field(scenarios, "/scenario_id", "operator_debug_repair_action_clarity")?;
	let operator_selected =
		find_by_field(scenarios, "/scenario_id", "operator_debug_selected_but_not_narrated")?;
	let expansion = find_by_field(scenarios, "/scenario_id", "query_expansion_attribution")?;
	let dense_sparse =
		find_by_field(scenarios, "/scenario_id", "dense_sparse_channel_attribution")?;
	let fusion = find_by_field(scenarios, "/scenario_id", "fusion_attribution")?;
	let rerank = find_by_field(scenarios, "/scenario_id", "rerank_attribution")?;
	let candidate_drop = find_by_field(scenarios, "/scenario_id", "candidate_drop_diagnostics")?;
	let selected =
		find_by_field(scenarios, "/scenario_id", "selected_but_not_narrated_wrong_results")?;
	let tombstone =
		find_by_field(scenarios, "/scenario_id", "evidence_absent_tombstone_diagnostics")?;

	assert_eq!(scenarios.len(), 16);
	assert_eq!(retrieval.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(top10.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(replay.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(trace_surface.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(
		operator_trace.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(operator_trace.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(operator_trace.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(operator_replay.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_candidate.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(array_contains_str(
		operator_candidate,
		"/typed_non_pass_states",
		"retrieved_but_dropped"
	)?);
	assert_eq!(operator_repair.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_selected.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(array_contains_str(
		operator_selected,
		"/typed_non_pass_states",
		"selected_but_not_narrated"
	)?);
	assert_eq!(expansion.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(dense_sparse.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(fusion.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(rerank.pointer("/result_type").and_then(Value::as_str), Some("non_goal"));
	assert_eq!(rerank.pointer("/outcome").and_then(Value::as_str), Some("non_goal"));
	assert_eq!(candidate_drop.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert!(array_contains_str(candidate_drop, "/typed_non_pass_states", "retrieved_but_dropped")?);
	assert_eq!(selected.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert!(array_contains_str(selected, "/typed_non_pass_states", "selected_but_not_narrated")?);
	assert_eq!(tombstone.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(tombstone.pointer("/qmd_status").and_then(Value::as_str), Some("wrong_result"));
	assert!(array_contains_str(
		report,
		"/wrong_result_diagnostics/qmd_missing_evidence",
		"delete-tombstone"
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"qmd currently wins the default local-debug artifact surface: top-10 rows plus short CLI replay."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"ELF narrowly wins the live operator-debug trace hydration and candidate-drop visibility slice against qmd; qmd still ties replay-command and repair-action clarity."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"Do not claim qmd beats ELF as a memory system overall."
	)?);

	Ok(())
}

fn assert_trace_replay_diagnostics_markdown(markdown: &str) {
	assert!(markdown.contains("Retrieval correctness is still tied"));
	assert!(markdown.contains("| Default top-10 candidate artifact |"));
	assert!(markdown.contains("| Replay command locality |"));
	assert!(
		markdown
			.contains("| Operator-debug trace hydration | `live_real_world` | `pass` | `win` |")
	);
	assert!(markdown.contains(
		"| Operator-debug replay command availability | `live_real_world` | `pass` | `tie` |"
	));
	assert!(markdown.contains(
		"| Operator-debug candidate-drop visibility | `live_real_world` | `pass` | `win` |"
	));
	assert!(markdown.contains("| Rerank attribution | `live_baseline_only` | `non_goal` |"));
	assert!(markdown.contains("| Candidate-drop diagnostics | `research_gate` | `not_encoded` |"));
	assert!(markdown.contains("`retrieved_but_dropped` | Defined globally as `not_tested`"));
	assert!(markdown.contains("npx tsx src/cli/qmd.ts query"));
	assert!(markdown.contains("cargo run -p elf-eval -- --config-a"));
	assert!(markdown.contains("cargo make real-world-job-operator-ux-live-adapters"));
	assert!(markdown.contains("Do not claim qmd beats ELF as a memory system overall"));
	assert!(markdown.contains("Do not score rerank superiority from a qmd `--no-rerank` run"));
}

fn assert_trace_replay_viewer_blocker_boundaries(
	readme: &str,
	markdown: &str,
	adoption_report: &str,
	report: &Value,
	adoption_json: &Value,
) -> Result<()> {
	let checked_surfaces = [
		collapse_whitespace(readme),
		collapse_whitespace(markdown),
		collapse_whitespace(adoption_report),
		report.to_string(),
		adoption_json.to_string(),
	];

	for surface in checked_surfaces {
		assert!(!surface.contains("blocked or not encoded"));
	}

	assert!(
		collapse_whitespace(readme)
			.contains("claude-mem viewer flows remain blocked until Docker-contained")
	);
	assert!(
		collapse_whitespace(markdown)
			.contains("claude-mem UI repair paths remain blocked until Docker-contained")
	);
	assert!(
		collapse_whitespace(adoption_report)
			.contains("claude-mem viewer workflows remain blocked until Docker-contained")
	);

	Ok(())
}

fn assert_trace_replay_adoption_json(adoption: &Value) -> Result<()> {
	let local_debug = find_by_field(
		array_at(adoption, "/scenario_outcomes")?,
		"/scenario_id",
		"local_debug_replay_ux",
	)?;
	let operator_debug = find_by_field(
		array_at(adoption, "/scenario_outcomes")?,
		"/scenario_id",
		"operator_debugging_viewer_ux",
	)?;

	assert_eq!(local_debug.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert!(
		local_debug
			.pointer("/measured_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd stronger on immediate top-10"))
	);
	assert!(array_contains_str(
		local_debug,
		"/command_artifacts",
		"docs/evidence/benchmarking/2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"
	)?);
	assert!(array_contains_str(
		adoption,
		"/claim_boundaries/not_allowed",
		"Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF memory-system or retrieval-quality win."
	)?);
	assert_eq!(operator_debug.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(
		operator_debug
			.pointer("/measured_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("narrow live operator-debug win over qmd"))
	);
	assert!(array_contains_str(
		operator_debug,
		"/command_artifacts",
		"tmp/real-world-job/operator-ux-live-adapters/summary.json"
	)?);
	assert!(array_contains_str(
		adoption,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF broadly beats OpenMemory or claude-mem viewer UX from the narrow ELF/qmd operator-debug slice."
	)?);

	Ok(())
}

fn assert_competitor_strength_matrix_json(matrix: &Value) -> Result<()> {
	let projects = array_at(matrix, "/project_matrix")?;
	let scenarios = array_at(matrix, "/scenario_matrix")?;

	assert_competitor_strength_matrix_manifest_counts(matrix);
	assert_competitor_strength_matrix_project_json(projects)?;
	assert_competitor_strength_matrix_scenario_json(scenarios)?;

	Ok(())
}

fn assert_competitor_strength_matrix_project_json(projects: &[Value]) -> Result<()> {
	let qmd = find_by_field(projects, "/project", "qmd")?;
	let mem0 = find_by_field(projects, "/project", "mem0/OpenMemory")?;
	let claude_mem = find_by_field(projects, "/project", "claude-mem")?;
	let openviking = find_by_field(projects, "/project", "OpenViking")?;

	assert_eq!(
		qmd.pointer("/current_evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(qmd.pointer("/measured_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert!(qmd.pointer("/benchmark_before_claim").and_then(Value::as_str).is_some_and(|claim| {
		claim.contains("Keep qmd deep retrieval/debug profiling separate")
			&& claim.contains("narrow operator-debug live slice")
	}));
	assert!(
		qmd.pointer("/borrow_if_stronger")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("transparent local knobs"))
	);
	assert_eq!(mem0.pointer("/measured_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		mem0.pointer("/unsupported_or_blocked_status/typed_reason").and_then(Value::as_str),
		Some("openmemory_export_helper_setup_blocked")
	);
	assert!(
		mem0.pointer("/benchmark_before_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("OpenMemory product app import/export"))
	);
	assert!(
		claude_mem
			.pointer("/unsupported_or_blocked_status/details")
			.and_then(Value::as_str)
			.is_some_and(|details| details.contains("rerun/inspection targets")
				&& details.contains("tmp/live-baseline/claude-mem-checks.json"))
	);
	assert_eq!(
		openviking.pointer("/current_evidence_class").and_then(Value::as_str),
		Some("live_baseline_only")
	);
	assert_eq!(
		openviking.pointer("/measured_status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		openviking.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("blocked")
	);
	assert!(
		openviking
			.pointer("/unsupported_or_blocked_status/details")
			.and_then(Value::as_str)
			.is_some_and(|details| details.contains("encoded as blocked fixtures"))
	);
	assert!(
		openviking
			.pointer("/benchmark_before_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("evidence-bearing same-corpus output pass"))
	);

	Ok(())
}

fn assert_competitor_strength_matrix_scenario_json(scenarios: &[Value]) -> Result<()> {
	let retrieval_debug = find_by_field(scenarios, "/scenario_id", "retrieval_debug")?;
	let work_resume = find_by_field(scenarios, "/scenario_id", "work_resume")?;
	let operator_debug = find_by_field(scenarios, "/scenario_id", "operator_debugging")?;
	let context_trajectory = find_by_field(scenarios, "/scenario_id", "context_trajectory")?;
	let consolidation = find_by_field(scenarios, "/scenario_id", "consolidation")?;

	assert!(
		retrieval_debug
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("Measured tie on encoded retrieval answers"))
	);
	assert!(retrieval_debug.pointer("/current_state").and_then(Value::as_str).is_some_and(
		|state| state.contains("qmd remains stronger on local debug ergonomics not fully scored")
	));
	assert!(
		work_resume
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("claude-mem work_resume remains not_encoded")
				&& !claim.contains("claude-mem is wrong_result"))
	);
	assert!(
		operator_debug
			.pointer("/current_elf_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("narrow live_real_world operator-debug slice"))
	);
	assert!(
		operator_debug
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd now has a narrow live_real_world"))
	);
	assert!(
		operator_debug
			.pointer("/next_measurement")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("OpenMemory and claude-mem UI/export"))
	);
	assert!(
		consolidation
			.pointer("/current_elf_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("XY-934 adds live_real_world")
				&& claim.contains("zero source mutations"))
	);
	assert!(
		consolidation
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd remains not_encoded")
				&& claim.contains("product references only"))
	);

	let personalization = find_by_field(scenarios, "/scenario_id", "personalization")?;

	assert_personalization_matrix_record(personalization);

	assert!(
		context_trajectory
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("not a measured live winner"))
	);
	assert!(
		context_trajectory
			.pointer("/next_measurement")
			.and_then(Value::as_str)
			.is_some_and(|measurement| measurement.contains("evidence-bearing retrieval pass"))
	);

	Ok(())
}

fn assert_personalization_matrix_record(personalization: &Value) {
	assert!(
		personalization
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim
				.contains("mem0/OpenMemory local OSS entity-scoped personalization now passes")
				&& claim.contains("Letta personalization is research_gate not_encoded"))
	);
	assert!(
		personalization
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("scoped personalization is a tie"))
	);
}

fn assert_competitor_strength_matrix_manifest_counts(matrix: &Value) {
	assert_eq!(
		matrix.pointer("/manifest_summary/adapter_records").and_then(Value::as_u64),
		Some(23)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/evidence_class_counts/live_real_world")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		matrix.pointer("/manifest_summary/overall_status_counts/pass").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		matrix.pointer("/manifest_summary/overall_status_counts/blocked").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/overall_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/overall_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);
}

fn assert_strength_profile_summary(report: &Value) {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.competitor_strength_profile_report/v1")
	);
	assert_eq!(
		report.pointer("/summary/qmd/retrieval_quality").and_then(Value::as_str),
		Some("tie")
	);
	assert_eq!(
		report.pointer("/summary/qmd/local_query_transparency").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/qmd/local_replayability").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/qmd/overall_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/openviking/overall_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/elf_win")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/qmd_strength_profile/win_tie_loss_summary/tie").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/elf_loss")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/elf_win")
			.and_then(Value::as_u64),
		Some(1)
	);
}

fn assert_strength_profile_terms(report: &Value) -> Result<()> {
	let result_terms = array_at(report, "/result_type_terms")?;
	let coverage_terms = array_at(report, "/coverage_status_terms")?;
	let outcome_terms = array_at(report, "/outcome_terms")?;
	let actual_result_terms = string_array_at(report, "/result_type_terms")?;
	let actual_coverage_terms = string_array_at(report, "/coverage_status_terms")?;

	assert_eq!(
		actual_result_terms,
		[
			"pass",
			"wrong_result",
			"blocked",
			"incomplete",
			"lifecycle_fail",
			"not_encoded",
			"unsupported_claim",
		]
		.map(str::to_owned)
	);
	assert_eq!(
		actual_coverage_terms,
		[
			"pass",
			"wrong_result",
			"blocked",
			"incomplete",
			"lifecycle_fail",
			"not_encoded",
			"unsupported",
			"unsupported_claim",
		]
		.map(str::to_owned)
	);
	assert!(!result_terms.iter().any(|term| term.as_str() == Some("unsupported")));
	assert!(!result_terms.iter().any(|term| term.as_str() == Some("partial")));
	assert!(!coverage_terms.iter().any(|term| term.as_str() == Some("partial")));
	assert!(result_terms.iter().any(|term| term.as_str() == Some("unsupported_claim")));
	assert!(coverage_terms.iter().any(|term| term.as_str() == Some("unsupported")));

	assert_value_in_terms(report, "/summary/qmd/overall_outcome", outcome_terms)?;
	assert_value_in_terms(report, "/summary/openviking/overall_outcome", outcome_terms)?;

	for scenario in array_at(report, "/qmd_strength_profile/scenario_outcomes")? {
		assert_value_in_terms(scenario, "/result_type", result_terms)?;
		assert_value_in_terms(scenario, "/elf_status", coverage_terms)?;
		assert_value_in_terms(scenario, "/qmd_status", coverage_terms)?;
	}
	for scenario in array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")? {
		assert_value_in_terms(scenario, "/result_type", result_terms)?;
		assert_value_in_terms(scenario, "/openviking_status", coverage_terms)?;
		assert_value_in_terms(scenario, "/elf_equivalent_status", coverage_terms)?;
	}

	Ok(())
}

fn assert_value_in_terms(value: &Value, pointer: &str, terms: &[Value]) -> Result<()> {
	let actual = value
		.pointer(pointer)
		.and_then(Value::as_str)
		.ok_or_else(|| eyre::eyre!("missing string at {pointer}"))?;

	assert!(
		terms.iter().any(|term| term.as_str() == Some(actual)),
		"{actual} at {pointer} is not declared in the report term list"
	);

	Ok(())
}

fn assert_qmd_strength_profile(report: &Value) -> Result<()> {
	let qmd_scenarios = array_at(report, "/qmd_strength_profile/scenario_outcomes")?;
	let local_transparency =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-local-query-transparency")?;
	let retrieval = find_by_field(qmd_scenarios, "/scenario_id", "qmd-retrieval-quality")?;
	let rerank_controls =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-expansion-fusion-rerank-controls")?;
	let stale_isolation =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-stale-context-isolation")?;
	let lifecycle = find_by_field(qmd_scenarios, "/scenario_id", "qmd-update-delete-cold-start")?;
	let operator_debug =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-operator-debug-evidence")?;
	let replayability = find_by_field(qmd_scenarios, "/scenario_id", "qmd-local-replayability")?;
	let wrong_result = find_by_field(qmd_scenarios, "/scenario_id", "qmd-wrong-result-diagnosis")?;

	assert_eq!(qmd_scenarios.len(), 8);
	assert_eq!(retrieval.pointer("/elf_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(
		local_transparency.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		local_transparency.pointer("/result_type").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(
		rerank_controls.pointer("/result_type").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(stale_isolation.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(stale_isolation.pointer("/elf_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(lifecycle.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(lifecycle.pointer("/elf_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_debug.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(operator_debug.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(replayability.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(replayability.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(
		wrong_result.pointer("/evidence_class").and_then(Value::as_str),
		Some("research_gate")
	);
	assert_eq!(wrong_result.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));

	Ok(())
}

fn assert_qmd_wrong_result_diagnosis(report: &Value) -> Result<()> {
	let taxonomy = array_at(report, "/qmd_strength_profile/wrong_result_diagnosis/taxonomy")?;
	let absent = find_by_field(taxonomy, "/class", "evidence_absent")?;
	let dropped = find_by_field(taxonomy, "/class", "retrieved_but_dropped")?;
	let narrated = find_by_field(taxonomy, "/class", "selected_but_not_narrated")?;
	let lifecycle = find_by_field(taxonomy, "/class", "contradicted_by_lifecycle_evidence")?;

	assert_eq!(absent.pointer("/coverage").and_then(Value::as_str), Some("observed"));
	assert_eq!(
		dropped.pointer("/coverage").and_then(Value::as_str),
		Some("not_observed_candidate_trace_missing")
	);
	assert_eq!(narrated.pointer("/coverage").and_then(Value::as_str), Some("observed"));
	assert_eq!(lifecycle.pointer("/coverage").and_then(Value::as_str), Some("observed"));

	let qmd_diagnosis_jobs = array_at(report, "/qmd_strength_profile/wrong_result_diagnosis/jobs")?;
	let delete_job =
		find_by_field(qmd_diagnosis_jobs, "/job_id", "memory-evolution-delete-ttl-001")?;

	assert_eq!(qmd_diagnosis_jobs.len(), 6);
	assert_eq!(delete_job.pointer("/qmd_status").and_then(Value::as_str), Some("wrong_result"));
	assert!(array_contains_str(delete_job, "/missing_evidence", "delete-tombstone")?);
	assert!(
		delete_job
			.pointer("/diagnosis")
			.and_then(Value::as_str)
			.is_some_and(|diagnosis| diagnosis.contains("typed wrong_result"))
	);

	Ok(())
}

fn assert_openviking_strength_profile(report: &Value) -> Result<()> {
	let openviking_scenarios =
		array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")?;
	let trajectory = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-staged-retrieval-trajectory",
	)?;
	let precondition = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-evidence-bearing-retrieval-precondition",
	)?;
	let local_embed_setup =
		find_by_field(openviking_scenarios, "/scenario_id", "openviking-local-embed-setup")?;
	let missed_terms = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-missed-expected-terms-evidence",
	)?;
	let hierarchy =
		find_by_field(openviking_scenarios, "/scenario_id", "openviking-hierarchy-selection")?;
	let recursive_expansion = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-recursive-context-expansion",
	)?;

	assert_eq!(openviking_scenarios.len(), 6);
	assert_eq!(
		trajectory.pointer("/evidence_class").and_then(Value::as_str),
		Some("fixture_backed")
	);
	assert_eq!(trajectory.pointer("/result_type").and_then(Value::as_str), Some("blocked"));
	assert_eq!(trajectory.pointer("/openviking_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(local_embed_setup.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		local_embed_setup.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(local_embed_setup.pointer("/typed_blocker"), Some(&Value::Null));
	assert_eq!(precondition.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(precondition.pointer("/elf_outcome").and_then(Value::as_str), Some("elf_win"));
	assert_eq!(
		precondition.pointer("/typed_blocker").and_then(Value::as_str),
		Some("output_missed_expected_terms")
	);
	assert_eq!(missed_terms.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(missed_terms.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(hierarchy.pointer("/result_type").and_then(Value::as_str), Some("blocked"));
	assert_eq!(hierarchy.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(
		recursive_expansion.pointer("/result_type").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		recursive_expansion.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);

	Ok(())
}

fn assert_strength_profile_json_claim_boundaries(report: &Value) -> Result<()> {
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not broadly beat qmd; it ties encoded retrieval and lifecycle correctness, keeps qmd query transparency as not_tested for comparative scoring, and leaves replayability not_tested."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"qmd expansion, fusion, and rerank superiority remains not_tested because the current qmd paths use --no-rerank and do not score internals."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not beat OpenViking on context trajectory; OpenViking trajectory strengths remain blocked/not_tested behind a wrong_result same-corpus output precondition and missing staged artifacts."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"Research_gate and blocked fixture records are follow-up gates, not pass evidence."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"Missing equivalent surfaces are encoded as unsupported, blocked, or not_encoded rather than fake losses."
	)?);

	Ok(())
}

fn assert_strength_profile_markdown_boundaries(markdown: &str) {
	assert!(
		markdown.contains(
			"| Wrong-result diagnosis | `research_gate` | `not_encoded` | `not_tested` |"
		)
	);
	assert!(
		markdown.contains("ELF ties qmd on the current encoded retrieval-correctness surfaces")
	);
	assert!(markdown.contains("qmd remains the local retrieval-debug UX reference"));
	assert!(markdown.contains("not scored as comparative ELF wins or losses"));
	assert!(markdown.contains("ELF currently wins only the equivalent OpenViking same-corpus"));
	assert!(markdown.contains("Do not claim ELF broadly beats qmd"));
	assert!(markdown.contains(
		"Do not claim ELF beats OpenViking on staged retrieval, hierarchy, or recursive"
	));
	assert!(markdown.contains(
		"Do not turn `research_gate`, `blocked`, `not_encoded`, or `unsupported` surfaces"
	));
	assert!(markdown.contains("no pass evidence is claimed"));
	assert!(markdown.contains("typed `wrong_result` state"));
}

fn assert_operator_facing_strength_profile_boundaries(
	readme: &str,
	benchmarking_index: &str,
	iteration_direction: &str,
) {
	assert!(readme.contains("Full-suite live real-world adapter sweep after XY-926"));
	assert!(readme.contains("all 55 checked-in jobs across 13 suites"));
	assert!(readme.contains("ELF now live-scores capture/write-policy"));
	assert!(readme.contains("consolidation proposal review"));
	assert!(readme.contains("knowledge-page rebuild/lint"));
	assert!(readme.contains("operator-debugging fixtures"));
	assert!(!readme.contains("memory-evolution wrong results"));
	assert!(readme.contains("Live temporal reconciliation after XY-905"));
	assert!(readme.contains("now reports ELF live `memory_evolution` as 6/6 pass"));
	assert!(readme.contains("broad qmd, Graphiti/Zep, mem0/OpenMemory, Letta"));
	assert!(readme.contains("production-ops operator boundaries"));
	assert!(readme.contains("core/archival live adapter gap"));
	assert!(collapse_whitespace(readme).contains("blocked context-trajectory measurement"));
	assert!(
		readme
			.contains("consolidation, knowledge, capture, and core/archival typed non-pass states")
	);
	assert!(readme.contains("operator-debug trace hydration"));
	assert!(readme.contains("qmd remains the local retrieval-debug UX reference"));
	assert!(readme.contains("broad ELF-over-qmd"));
	assert!(readme.contains("qmd and OpenViking Strength-Profile Report - June 11, 2026"));
	assert!(benchmarking_index.contains("2026-06-11-qmd-openviking-strength-profile-report.md"));
	assert!(
		benchmarking_index.contains("separates qmd retrieval quality from debug/replay ergonomics")
	);
	assert!(benchmarking_index.contains("preserves XY-928 OpenViking"));
	assert!(
		benchmarking_index
			.contains("context-trajectory surfaces as blocked/not-tested until scored staged")
	);
	assert!(
		iteration_direction
			.contains("ELF and qmd are tied on the encoded live retrieval, work-resume, and")
	);
	assert!(iteration_direction.contains("ELF does not yet beat qmd's local retrieval-debug"));

	assert_iteration_direction_current_measurement_counts(iteration_direction);

	assert!(iteration_direction.contains(
		"ELF beats OpenViking on context trajectory. The scenario is encoded as blocked"
	));
	assert!(
		iteration_direction
			.contains("Do not promote a reference project into a win/loss claim until")
	);
}

fn assert_measurement_audit_adapter_status_counts(markdown: &str) {
	for expected in [
		"| `blocked` | `7` |",
		"| `not_encoded` | `5` |",
		"The generated JSON report emits `external_project_count: 16`",
	] {
		assert!(markdown.contains(expected), "missing measurement audit text: {expected}");
	}
	for stale in ["| `blocked` | `6` |", "| `not_encoded` | `6` |"] {
		assert!(!markdown.contains(stale), "stale measurement audit text: {stale}");
	}
}

fn assert_iteration_direction_current_measurement_counts(markdown: &str) {
	for expected in [
		"| Jobs | `55` |",
		"| Encoded suites | `15` |",
		"| Blocked | `6` |",
		"| Mean score | `0.891` |",
		"| Evidence coverage | `123/123` |",
		"| Source-ref coverage | `123/123` |",
		"| Quote coverage | `123/123` |",
		"| Expected evidence recall | `115/115` |",
		"| `blocked` | `7` |",
		"| `not_encoded` | `5` |",
		"`live_baseline_only`, `fixture_backed`, and `research_gate`",
		"`blocked` for fixture-backed trajectory gates",
	] {
		assert!(markdown.contains(expected), "missing iteration-direction text: {expected}");
	}
	for stale in [
		"| Jobs | `40` |",
		"| Encoded suites | `11` |",
		"| Jobs | `50` |",
		"| Encoded suites | `14` |",
		"| Mean score | `0.950` |",
		"| Mean score | `0.900` |",
		"| Evidence coverage | `88/88` |",
		"| Evidence coverage | `115/115` |",
		"| Expected evidence recall | `80/80` |",
		"| Expected evidence recall | `107/107` |",
		"| `blocked` | `5` |",
		"| `not_encoded` | `7` |",
		"`live_baseline_only` plus `research_gate`",
	] {
		assert!(!markdown.contains(stale), "stale iteration-direction text: {stale}");
	}
}

#[test]
fn generated_json_report_renders_markdown() -> Result<()> {
	let report = run_json_report()?;
	let temp_dir = env::temp_dir().join(format!("elf-real-world-job-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("report.json");
	let markdown_path = temp_dir.join("report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("# Real-World Job Benchmark Report"));
	assert!(markdown.contains("work_resume"));
	assert!(markdown.contains("Capture And Integration Coverage"));
	assert!(markdown.contains("External Adapter Coverage"));
	assert!(markdown.contains("live-baseline-only"));
	assert!(markdown.contains("live real-world"));
	assert!(markdown.contains("does not convert live-baseline retrieval results"));
	assert!(markdown.contains("fixture-backed"));
	assert!(markdown.contains("Answer Type"));
	assert!(markdown.contains("Caveat Required"));
	assert!(markdown.contains("Refusal Required"));
	assert!(markdown.contains("agentmemory-style hook capture"));
	assert!(markdown.contains("xy844-current-worktree"));
	assert!(markdown.contains("Existing live-baseline reports remain valid"));
	assert!(markdown.contains("### Adapter Scenario Judgments"));
	assert!(markdown.contains("ELF scenario positions: `wins=10, ties=11, loses=1, untested=35`"));
	assert!(markdown.contains(
		"Scenario comparison outcomes: `win=10, tie=11, loss=1, not_tested=13, blocked=17, non_goal=5`"
	));
	assert!(markdown.contains("| `claude_mem_live_baseline` | `same_corpus_retrieval`"));
	assert!(markdown.contains("| `memsearch_live_baseline` | `ttl_expiry_lifecycle`"));

	Ok(())
}

#[test]
fn external_adapter_markdown_renders_nonzero_scenario_losses() -> Result<()> {
	let mut report = run_json_report()?;
	let adapters = report
		.pointer_mut("/external_adapters/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing external adapter records"))?;
	let adapter = adapters
		.iter_mut()
		.find(|adapter| {
			adapter.pointer("/adapter_id").and_then(Value::as_str)
				== Some("agentmemory_live_baseline")
		})
		.ok_or_else(|| eyre::eyre!("missing agentmemory adapter"))?;

	set_json_pointer(adapter, "/scenarios/0/elf_position", serde_json::json!("loses"))?;
	set_json_pointer(adapter, "/scenarios/0/comparison_outcome", serde_json::json!("loss"))?;
	set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_position_counts",
		serde_json::json!({
			"wins": 2,
			"ties": 4,
			"loses": 2,
			"untested": 10
		}),
	)?;
	set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_outcome_counts",
		serde_json::json!({
			"win": 2,
			"tie": 4,
			"loss": 2,
			"not_tested": 7,
			"blocked": 1,
			"non_goal": 2
		}),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-loss-scenario-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("report.json");
	let markdown_path = temp_dir.join("report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("ELF scenario positions: `wins=2, ties=4, loses=2, untested=10`"));
	assert!(markdown.contains(
		"Scenario comparison outcomes: `win=2, tie=4, loss=2, not_tested=7, blocked=1, non_goal=2`"
	));
	assert!(markdown.contains(
		"| `agentmemory_live_baseline` | `basic_same_corpus_retrieval` | `retrieval` | `pass` | `loss` |"
	));

	Ok(())
}

#[test]
fn external_adapter_markdown_omits_scenario_summary_when_manifest_has_no_scenarios() -> Result<()> {
	let mut report = run_json_report()?;
	let adapters = report
		.pointer_mut("/external_adapters/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing external adapter records"))?;

	for adapter in adapters {
		set_json_pointer(adapter, "/scenarios", serde_json::json!([]))?;
	}

	set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_status_counts",
		serde_json::json!({
			"real": 0,
			"mocked": 0,
			"unsupported": 0,
			"blocked": 0,
			"incomplete": 0,
			"wrong_result": 0,
			"lifecycle_fail": 0,
			"pass": 0,
			"not_encoded": 0
		}),
	)?;
	set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_position_counts",
		serde_json::json!({
			"wins": 0,
			"ties": 0,
			"loses": 0,
			"untested": 0
		}),
	)?;
	set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_outcome_counts",
		serde_json::json!({
			"win": 0,
			"tie": 0,
			"loss": 0,
			"not_tested": 0,
			"blocked": 0,
			"non_goal": 0
		}),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-no-scenario-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("report.json");
	let markdown_path = temp_dir.join("report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("External Adapter Coverage"));
	assert!(!markdown.contains("Scenario coverage statuses:"));
	assert!(!markdown.contains("ELF scenario positions:"));
	assert!(!markdown.contains("Scenario comparison outcomes:"));
	assert!(!markdown.contains("### Adapter Scenario Judgments"));

	Ok(())
}

#[test]
fn mem0_delete_audit_probe_requires_explicit_delete_history_event() -> Result<()> {
	let script =
		fs::read_to_string(workspace_root()?.join("scripts").join("live-baseline-benchmark.sh"))?;

	assert!(script.contains("def history_has_event"));
	assert!(script.contains("str(entry.get(\"event\", \"\")).upper() == expected"));
	assert!(script.contains(
		"history_has_event(\n        preference_history[\"history\"],\n        \"ADD\","
	));
	assert!(script.contains(
		"history_has_event(\n        preference_history[\"history\"],\n        \"UPDATE\","
	));
	assert!(
		script.contains(
			"history_has_event(\n        delete_history[\"history\"],\n        \"DELETE\","
		)
	);
	assert!(
		!script.contains(
			"contains_terms(\n        delete_history[\"history\"],\n        [\"delete\"],"
		)
	);

	Ok(())
}

#[test]
fn dreaming_readiness_stage_ledger_preserves_gate_shape() -> Result<()> {
	let ledger = serde_json::from_str::<Value>(&fs::read_to_string(
		dreaming_readiness_stage_ledger_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(dreaming_readiness_stage_ledger_markdown_path()?)?;
	let stages = array_at(&ledger, "/stage_gates")?;

	assert_dreaming_readiness_ledger_header(&ledger)?;
	assert_dreaming_readiness_stage_shape(&ledger, stages)?;
	assert_dreaming_readiness_baseline_counts(&ledger, stages)?;
	assert_dreaming_readiness_markdown_boundaries(&markdown);

	Ok(())
}

fn assert_dreaming_readiness_ledger_header(ledger: &Value) -> Result<()> {
	assert_eq!(
		ledger.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_readiness_stage_ledger/v1")
	);
	assert_eq!(ledger.pointer("/authority").and_then(Value::as_str), Some("XY-951"));

	for term in ["improved", "regressed", "unchanged", "blocked", "not_tested"] {
		assert!(array_contains_str(ledger, "/judgment_terms", term)?);
	}
	for term in ["pass", "wrong_result", "blocked", "not_tested", "not_encoded"] {
		assert!(array_contains_str(ledger, "/count_fields", term)?);
	}

	Ok(())
}

fn assert_dreaming_readiness_stage_shape(ledger: &Value, stages: &[Value]) -> Result<()> {
	assert_eq!(stages.len(), 8);

	for stage_id in [
		"current_vs_historical_correctness",
		"preference_evolution",
		"deletion_ttl_tombstone_behavior",
		"reviewable_consolidation",
		"memory_summary_top_of_mind_behavior",
		"proactive_brief_readiness",
		"scheduled_memory_task_readiness",
		"final_competitor_retest_status",
	] {
		find_by_field(stages, "/stage_id", stage_id)?;
	}
	for stage in stages {
		let stage_id =
			stage.pointer("/stage_id").and_then(Value::as_str).unwrap_or("<missing stage_id>");

		assert!(
			!array_at(stage, "/baseline_commands")?.is_empty(),
			"{stage_id} missing baseline commands"
		);
		assert!(
			!array_at(stage, "/post_stage_commands")?.is_empty(),
			"{stage_id} missing post-stage commands"
		);
		assert!(
			!array_at(stage, "/evidence_files")?.is_empty(),
			"{stage_id} missing evidence files"
		);

		for count_field in string_array_at(ledger, "/count_fields")? {
			let pointer = format!("/baseline_counts/{count_field}");

			assert!(
				stage.pointer(&pointer).and_then(Value::as_u64).is_some(),
				"{stage_id} missing {pointer}"
			);
		}

		let judgment = stage
			.pointer("/comparison_judgment")
			.and_then(Value::as_str)
			.ok_or_else(|| eyre::eyre!("{stage_id} missing comparison_judgment"))?;

		assert!(array_contains_str(ledger, "/judgment_terms", judgment)?);
	}

	Ok(())
}

fn assert_dreaming_readiness_baseline_counts(ledger: &Value, stages: &[Value]) -> Result<()> {
	let current = find_by_field(stages, "/stage_id", "current_vs_historical_correctness")?;

	assert_eq!(current.pointer("/baseline_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(current.pointer("/baseline_counts/wrong_result").and_then(Value::as_u64), Some(5));
	assert_eq!(current.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(current.pointer("/post_stage_counts/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(current.pointer("/comparison_judgment").and_then(Value::as_str), Some("improved"));
	assert!(
		current
			.pointer("/baseline_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("five current-vs-historical jobs"))
	);
	assert!(
		current
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("passes all six encoded jobs"))
	);

	let preference = find_by_field(stages, "/stage_id", "preference_evolution")?;

	assert_eq!(
		preference.pointer("/baseline_counts/wrong_result").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(preference.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(
		preference.pointer("/post_stage_counts/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		preference.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);

	let tombstone = find_by_field(stages, "/stage_id", "deletion_ttl_tombstone_behavior")?;

	assert_eq!(tombstone.pointer("/baseline_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(tombstone.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(
		tombstone.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(
		tombstone
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("tombstone and invalidation evidence"))
	);

	let consolidation = find_by_field(stages, "/stage_id", "reviewable_consolidation")?;

	assert_eq!(
		consolidation.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(
		consolidation.pointer("/baseline_counts/not_encoded").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(consolidation.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(
		consolidation.pointer("/post_stage_counts/not_encoded").and_then(Value::as_u64),
		Some(0)
	);
	assert!(
		consolidation
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("apply/defer/discard audit")
				&& basis.contains("zero source mutations"))
	);

	let scheduled = find_by_field(stages, "/stage_id", "scheduled_memory_task_readiness")?;

	assert_eq!(scheduled.pointer("/comparison_judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(scheduled.pointer("/baseline_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(scheduled.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(scheduled.pointer("/post_stage_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(
		scheduled.pointer("/post_stage_counts/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		scheduled.pointer("/post_stage_counts/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	assert_dreaming_final_competitor_retest_stage(ledger, stages)?;
	assert_dreaming_memory_summary_stage(stages)?;
	assert_dreaming_proactive_brief_stage(stages)?;

	Ok(())
}

fn assert_dreaming_final_competitor_retest_stage(ledger: &Value, stages: &[Value]) -> Result<()> {
	let retest = find_by_field(stages, "/stage_id", "final_competitor_retest_status")?;

	assert_eq!(retest.pointer("/baseline_counts/pass").and_then(Value::as_u64), Some(22));
	assert_eq!(retest.pointer("/baseline_counts/wrong_result").and_then(Value::as_u64), Some(5));
	assert_eq!(retest.pointer("/baseline_counts/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(retest.pointer("/baseline_counts/not_tested").and_then(Value::as_u64), Some(11));
	assert_eq!(retest.pointer("/baseline_counts/not_encoded").and_then(Value::as_u64), Some(11));
	assert_eq!(retest.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(40));
	assert_eq!(retest.pointer("/post_stage_counts/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(retest.pointer("/post_stage_counts/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(retest.pointer("/post_stage_counts/not_encoded").and_then(Value::as_u64), Some(19));
	assert_eq!(retest.pointer("/qmd_post_stage_counts/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(
		retest.pointer("/qmd_post_stage_counts/wrong_result").and_then(Value::as_u64),
		Some(13)
	);
	assert!(retest.pointer("/post_stage_basis").and_then(Value::as_str).is_some_and(|basis| {
		basis.contains("XY-955 closeout retest")
			&& basis.contains("qmd live adapter materialization is 17 pass")
	}));

	assert_dreaming_readiness_summary_buckets(ledger)
}

fn assert_dreaming_readiness_summary_buckets(ledger: &Value) -> Result<()> {
	assert!(array_contains_str(ledger, "/summary/improved", "current_vs_historical_correctness")?);
	assert!(array_contains_str(ledger, "/summary/improved", "preference_evolution")?);
	assert!(array_contains_str(ledger, "/summary/improved", "reviewable_consolidation")?);
	assert!(array_contains_str(
		ledger,
		"/summary/improved",
		"memory_summary_top_of_mind_behavior"
	)?);
	assert!(array_contains_str(ledger, "/summary/improved", "proactive_brief_readiness")?);
	assert!(array_contains_str(ledger, "/summary/improved", "scheduled_memory_task_readiness")?);
	assert!(array_at(ledger, "/summary/regressed")?.is_empty());
	assert!(array_contains_str(ledger, "/summary/unchanged", "deletion_ttl_tombstone_behavior")?);
	assert!(array_contains_str(ledger, "/summary/unchanged", "final_competitor_retest_status")?);
	assert!(array_at(ledger, "/summary/blocked")?.is_empty());
	assert!(array_at(ledger, "/summary/not_tested")?.is_empty());

	Ok(())
}

fn assert_dreaming_memory_summary_stage(stages: &[Value]) -> Result<()> {
	let summary_stage = find_by_field(stages, "/stage_id", "memory_summary_top_of_mind_behavior")?;

	assert_eq!(
		summary_stage.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(summary_stage.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(9));
	assert_eq!(
		summary_stage.pointer("/post_stage_counts/not_tested").and_then(Value::as_u64),
		Some(0)
	);
	assert!(
		summary_stage
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("fixture-backed memory_summary job")
				&& basis.contains("unsupported-claim flags"))
	);

	Ok(())
}

fn assert_dreaming_proactive_brief_stage(stages: &[Value]) -> Result<()> {
	let proactive_stage = find_by_field(stages, "/stage_id", "proactive_brief_readiness")?;

	assert_eq!(
		proactive_stage.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(proactive_stage.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(
		proactive_stage.pointer("/post_stage_counts/blocked").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		proactive_stage.pointer("/post_stage_counts/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		proactive_stage.pointer("/post_stage_counts/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		proactive_stage
			.pointer("/post_stage_counts/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		proactive_stage
			.pointer("/post_stage_counts/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert!(
		proactive_stage
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("five proactive_brief fixture jobs")
				&& basis.contains("typed private-corpus refresh blocker"))
	);

	Ok(())
}

fn assert_dreaming_readiness_markdown_boundaries(markdown: &str) {
	assert!(
		markdown.contains("`improved`: current-vs-historical correctness, preference evolution")
			&& markdown.contains("reviewable")
			&& markdown.contains("proactive brief")
	);
	assert!(markdown.contains("memory-summary/top-of-mind fixture readback"));
	assert!(markdown.contains("XY-953 adds a direct `proactive_brief` suite"));
	assert!(markdown.contains("XY-954 adds a direct `scheduled_memory` suite"));
	assert!(markdown.contains(
		"Do not claim fixture-backed proactive brief scoring proves OpenAI Pulse parity"
	));
	assert!(
		markdown
			.contains("Do not claim fixture-backed scheduled-memory scoring proves ChatGPT Tasks")
	);
	assert!(markdown.contains("`regressed`: none"));
	assert!(markdown.contains("the XY-905 run passes all six memory-evolution jobs"));
	assert!(markdown.contains("XY-952 adds a reviewable `elf.memory_summary/v1`"));
	assert!(markdown.contains("XY-955 closes the final competitor retest row"));
	assert!(markdown.contains("XY-905"));
	assert!(markdown.contains("qmd live `pass=17`, `wrong_result=13`"));
	assert!(
		markdown
			.contains("Do not claim this ledger proves preference history against mem0/OpenMemory")
	);
	assert!(markdown.contains("Reviewable consolidation now has ELF live service-backed"));
}

#[test]
fn knowledge_json_report_renders_markdown_metrics() -> Result<()> {
	let report = run_json_report_from(knowledge_fixture_dir())?;
	let temp_dir = env::temp_dir().join(format!("elf-real-world-knowledge-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("knowledge-report.json");
	let markdown_path = temp_dir.join("knowledge-report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("Knowledge Page Metrics"));
	assert!(markdown.contains("Knowledge citation coverage"));
	assert!(markdown.contains("Backlinks: `9` total"));
	assert!(markdown.contains("Unsupported summary count"));
	assert!(markdown.contains("knowledge-project-page-001"));
	assert!(markdown.contains("knowledge-entity-concept-002"));

	Ok(())
}

#[test]
fn memory_summary_fixtures_score_reviewable_source_trace_contract() -> Result<()> {
	let report = run_json_report_from(memory_summary_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/memory_summary/summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/entry_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/summary/memory_summary/covered_required_category_count")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/rationale_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/memory_summary/unsupported_derived_entry_count")
			.and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(&report, "/suites")?;
	let memory_summary = find_by_field(suites, "/suite_id", "memory_summary")?;

	assert_eq!(memory_summary.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(memory_summary.pointer("/encoded_job_count").and_then(Value::as_u64), Some(1));

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(job.pointer("/memory_summary/top_of_mind_count").and_then(Value::as_u64), Some(1));
	assert_eq!(job.pointer("/memory_summary/tombstone_ref_count").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn memory_summary_markdown_renders_source_trace_metrics() -> Result<()> {
	let report = run_json_report_from(memory_summary_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-summary-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("memory-summary-report.json");
	let markdown_path = temp_dir.join("memory-summary-report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("Memory Summary Metrics"));
	assert!(markdown.contains("memory-summary-source-trace-001"));
	assert!(markdown.contains("Memory summary source-ref coverage"));
	assert!(markdown.contains("Invalid Top-of-Mind"));
	assert!(markdown.contains("Derived Unsupported"));

	Ok(())
}

#[test]
fn memory_summary_fixture_fails_stale_top_of_mind_entries() -> Result<()> {
	let fixture_path = memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][2]["category"] =
		Value::String("top_of_mind".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][2]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir =
		env::temp_dir().join(format!("elf-memory-summary-stale-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stale_current_summary.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn memory_summary_fixture_fails_tombstoned_top_of_mind_entries() -> Result<()> {
	let fixture_path = memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][4]["category"] =
		Value::String("top_of_mind".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][4]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir = env::temp_dir()
		.join(format!("elf-memory-summary-tombstone-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("tombstone_current_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn memory_summary_fixture_fails_untraced_derived_profile_entries() -> Result<()> {
	let fixture_path = memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["unsupported_claim_flags"] =
		Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-memory-summary-untraced-derived-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("untraced_derived_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("unsupported_claim"));
	assert_eq!(
		job.pointer("/memory_summary/derived_missing_source_or_unsupported_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn memory_summary_fixture_fails_unsupported_current_derived_entries() -> Result<()> {
	let fixture_path = memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["source_refs"] =
		Value::Array(vec![Value::String("summary-contract-non-parity-boundary".to_string())]);
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["freshness"]
		["status"] = Value::String("current".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][6]["rationale"]
		["decision"] = Value::String("included".to_string());

	let temp_dir = env::temp_dir()
		.join(format!("elf-memory-summary-unsupported-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("unsupported_current_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/unsupported_current_entry_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn memory_summary_fixture_fails_tombstone_entries_without_tombstone_refs() -> Result<()> {
	let fixture_path = memory_summary_fixture_dir().join("reviewable_summary_source_trace.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["memory_summaries"][0]["entries"][4]["freshness"]
		["tombstone_refs"] = Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-memory-summary-tombstone-refs-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("missing_tombstone_refs_summary.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/memory_summary/freshness_coverage").and_then(Value::as_f64),
		Some(0.857)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn proactive_brief_fixtures_score_source_linked_suggestions() -> Result<()> {
	let report = run_json_report_from(proactive_brief_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/proactive_brief/brief_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/suggestion_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/invalid_current_suggestion_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/rejected_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/deferred_count").and_then(Value::as_u64),
		Some(2)
	);

	let suites = array_at(&report, "/suites")?;
	let proactive = find_by_field(suites, "/suite_id", "proactive_brief")?;

	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(proactive.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let jobs = array_at(&report, "/jobs")?;
	let daily = find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private = find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;

	assert_eq!(daily.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		daily.pointer("/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(private.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		report
			.pointer("/follow_ups/0/title")
			.and_then(Value::as_str)
			.is_some_and(|title| title.contains("XY-930"))
	);

	Ok(())
}

#[test]
fn proactive_brief_markdown_renders_source_and_freshness_metrics() -> Result<()> {
	let report = run_json_report_from(proactive_brief_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-proactive-brief-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("proactive-brief-report.json");
	let markdown_path = temp_dir.join("proactive-brief-report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("Proactive Brief Metrics"));
	assert!(markdown.contains("proactive-daily-project-brief-001"));
	assert!(markdown.contains("Proactive evidence-ref coverage"));
	assert!(markdown.contains("Invalid Current"));
	assert!(markdown.contains("Tombstone Violations"));

	Ok(())
}

#[test]
fn proactive_brief_fixture_fails_unsupported_suggestions() -> Result<()> {
	let fixture_path = proactive_brief_fixture_dir().join("daily_project_brief.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["evidence_refs"] =
		Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-proactive-unsupported-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("unsupported_brief.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("unsupported_claim"));
	assert_eq!(
		job.pointer("/proactive_brief/untraced_suggestion_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn proactive_brief_fixture_fails_stale_decisions_presented_current() -> Result<()> {
	let fixture_path = proactive_brief_fixture_dir().join("stale_decision_audit.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir =
		env::temp_dir().join(format!("elf-proactive-stale-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stale_current_brief.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "proactive-stale-decision-audit-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/proactive_brief/invalid_current_suggestion_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn proactive_brief_fixture_fails_tombstone_ttl_violations() -> Result<()> {
	let fixture_path = proactive_brief_fixture_dir().join("stale_plan_preference_warning.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["freshness"]
		["status"] = Value::String("current".to_string());
	fixture["corpus"]["adapter_response"]["answer"]["proactive_briefs"][0]["suggestions"][0]["action"]
		["decision"] = Value::String("recommend".to_string());

	let temp_dir = env::temp_dir().join(format!("elf-proactive-tombstone-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("tombstone_current_brief.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "proactive-stale-plan-preference-warning-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/proactive_brief/tombstone_violation_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn scheduled_memory_fixtures_score_task_trace_gate() -> Result<()> {
	let report = run_json_report_from(scheduled_memory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/scheduled_memory/job_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/task_run_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/output_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/invalid_current_output_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	let suites = array_at(&report, "/suites")?;
	let scheduled = find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let jobs = array_at(&report, "/jobs")?;
	let weekly = find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;
	let private =
		find_by_field(jobs, "/job_id", "scheduled-private-provider-scheduler-blocked-001")?;

	assert_eq!(weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		weekly.pointer("/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(private.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		report
			.pointer("/follow_ups/0/title")
			.and_then(Value::as_str)
			.is_some_and(|title| title.contains("XY-930"))
	);

	Ok(())
}

#[test]
fn scheduled_memory_markdown_renders_trace_metrics() -> Result<()> {
	let report = run_json_report_from(scheduled_memory_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-scheduled-memory-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("scheduled-memory-report.json");
	let markdown_path = temp_dir.join("scheduled-memory-report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("Scheduled Memory Metrics"));
	assert!(markdown.contains("scheduled-weekly-project-status-summary-001"));
	assert!(markdown.contains("Scheduled memory evidence-ref coverage"));
	assert!(markdown.contains("Trace Coverage"));
	assert!(markdown.contains("Source Mutations"));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_missing_execution_trace() -> Result<()> {
	let fixture_path = scheduled_memory_fixture_dir().join("weekly_project_status_summary.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]
		.as_object_mut()
		.ok_or_else(|| eyre::eyre!("missing scheduled task object"))?
		.remove("execution_trace");

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-missing-trace-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("missing_trace.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/scheduled_memory/trace_complete_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_untraced_outputs() -> Result<()> {
	let fixture_path = scheduled_memory_fixture_dir().join("weekly_project_status_summary.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["outputs"][0]["evidence_refs"] =
		Value::Array(Vec::new());

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-untraced-output-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("untraced_output.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("unsupported_claim"));
	assert_eq!(
		job.pointer("/scheduled_memory/untraced_output_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_superseded_sources_presented_current() -> Result<()> {
	let fixture_path = scheduled_memory_fixture_dir().join("stale_decision_audit.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["outputs"][0]["evidence_refs"] =
		serde_json::json!(["scheduled-old-consolidation-only-decision"]);
	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["outputs"][0]["freshness"]
		["status"] = Value::String("current".to_string());

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-superseded-current-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("superseded_current.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "scheduled-stale-decision-audit-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/scheduled_memory/invalid_current_output_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn scheduled_memory_fixture_fails_source_mutation() -> Result<()> {
	let fixture_path = scheduled_memory_fixture_dir().join("weekly_project_status_summary.json");
	let mut fixture = load_json(&fixture_path)?;

	fixture["corpus"]["adapter_response"]["answer"]["scheduled_tasks"][0]["source_mutations"] = serde_json::json!([
		{
			"table": "memory_notes",
			"op": "update",
			"note_id": "scheduled-weekly-current-gate"
		}
	]);

	let temp_dir =
		env::temp_dir().join(format!("elf-scheduled-source-mutation-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("source_mutation.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("lifecycle_fail"));
	assert_eq!(
		job.pointer("/scheduled_memory/source_mutation_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(report.pointer("/summary/lifecycle_fail").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn production_ops_fixtures_report_bounded_typed_states() -> Result<()> {
	let report = run_json_report_from(production_ops_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_case_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/private_corpus_redaction/private_fixture_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(&report, "/suites")?;
	let production_ops = find_by_field(suites, "/suite_id", "production_ops")?;

	assert_eq!(production_ops.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(production_ops.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let jobs = array_at(&report, "/jobs")?;
	let backfill = find_by_field(jobs, "/job_id", "production-ops-backfill-resume-001")?;
	let restore = find_by_field(jobs, "/job_id", "production-ops-restore-cold-start-001")?;
	let private_manifest =
		find_by_field(jobs, "/job_id", "production-ops-private-manifest-blocked-001")?;
	let credentials = find_by_field(jobs, "/job_id", "production-ops-credential-boundary-001")?;
	let dependency = find_by_field(jobs, "/job_id", "production-ops-cold-start-dependency-001")?;

	assert_eq!(backfill.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(restore.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(restore.pointer("/qdrant_rebuild_case").and_then(Value::as_bool), Some(true));
	assert_eq!(private_manifest.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(credentials.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(dependency.pointer("/status").and_then(Value::as_str), Some("pass"));

	Ok(())
}

#[test]
fn core_archival_memory_fixtures_score_separate_core_and_archival_jobs() -> Result<()> {
	let report = run_json_report_from(core_archival_memory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/evidence_required_count").and_then(Value::as_u64),
		Some(14)
	);
	assert_eq!(report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64), Some(14));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_violation_count").and_then(Value::as_u64), Some(0));

	let suites = array_at(&report, "/suites")?;
	let core = find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let jobs = array_at(&report, "/jobs")?;

	for job_id in [
		"core-archival-core-block-attachment-001",
		"core-archival-core-block-scope-001",
		"core-archival-core-block-provenance-001",
		"core-archival-stale-core-detection-001",
		"core-archival-archival-fallback-001",
		"core-archival-project-decision-recovery-001",
	] {
		let job = find_by_field(jobs, "/job_id", job_id)?;

		assert_eq!(job.pointer("/suite_id").and_then(Value::as_str), Some("core_archival_memory"));
		assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	}

	let scope = find_by_field(jobs, "/job_id", "core-archival-core-block-scope-001")?;
	let decision = find_by_field(jobs, "/job_id", "core-archival-project-decision-recovery-001")?;

	assert_eq!(scope.pointer("/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(scope.pointer("/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(scope.pointer("/scope_violation_count").and_then(Value::as_u64), Some(0));
	assert!(
		decision
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|content| content.contains("Letta remains blocked or not_tested"))
	);
	assert!(
		array_at(decision, "/produced_evidence")?
			.iter()
			.any(|id| id.as_str() == Some("decision-letta-export-boundary"))
	);

	Ok(())
}

#[test]
fn memory_authority_benchmark_covers_entity_history_and_core_archive_strengths() -> Result<()> {
	let report = run_json_report_from(real_world_memory_fixture_dir())?;

	assert_eq!(
		report.pointer("/summary/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(&report, "/suites")?;
	let memory_evolution = find_by_field(suites, "/suite_id", "memory_evolution")?;
	let core_archival = find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_archival.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memory_evolution.pointer("/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(core_archival.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let jobs = array_at(&report, "/jobs")?;
	let preference = find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;
	let core_attachment =
		find_by_field(jobs, "/job_id", "core-archival-core-block-attachment-001")?;
	let archival_fallback = find_by_field(jobs, "/job_id", "core-archival-archival-fallback-001")?;

	assert_eq!(preference.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		preference.pointer("/evolution/history_readback_encoded").and_then(Value::as_bool),
		Some(true)
	);
	assert!(array_contains_str(preference, "/evolution/history_event_types", "update")?);
	assert_eq!(core_attachment.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(archival_fallback.pointer("/status").and_then(Value::as_str), Some("pass"));

	let adapters = array_at(&report, "/external_adapters/adapters")?;
	let mem0 = find_by_field(adapters, "/adapter_id", "mem0_openmemory_live_baseline")?;
	let letta = find_by_field(adapters, "/adapter_id", "letta_research_gate")?;
	let mem0_scenarios = array_at(mem0, "/scenarios")?;
	let mem0_history =
		find_by_field(mem0_scenarios, "/scenario_id", "preference_correction_history")?;
	let mem0_entity =
		find_by_field(mem0_scenarios, "/scenario_id", "entity_scoped_personalization")?;

	assert_eq!(mem0_history.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0_entity.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0_history.pointer("/comparison_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(mem0_entity.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));

	let letta_scenarios = array_at(letta, "/scenarios")?;
	let letta_core =
		find_by_field(letta_scenarios, "/scenario_id", "core_block_attachment_readback")?;
	let letta_fallback =
		find_by_field(letta_scenarios, "/scenario_id", "archival_fallback_readback")?;

	for scenario in [letta_core, letta_fallback] {
		assert_eq!(
			scenario.pointer("/suite_id").and_then(Value::as_str),
			Some("core_archival_memory")
		);
		assert_eq!(scenario.pointer("/status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(
			scenario.pointer("/comparison_outcome").and_then(Value::as_str),
			Some("blocked")
		);
	}

	Ok(())
}

#[test]
fn context_trajectory_fixtures_report_blocked_openviking_gates() -> Result<()> {
	let report = run_json_report_from(context_trajectory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);

	let suites = array_at(&report, "/suites")?;
	let context = find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(context.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(context.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = array_at(&report, "/jobs")?;
	let staged =
		find_by_field(jobs, "/job_id", "context-trajectory-openviking-staged-retrieval-001")?;
	let hierarchy =
		find_by_field(jobs, "/job_id", "context-trajectory-openviking-hierarchy-selection-001")?;
	let recursive =
		find_by_field(jobs, "/job_id", "context-trajectory-openviking-recursive-expansion-001")?;

	assert_eq!(staged.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(hierarchy.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(recursive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		staged.pointer("/reason").and_then(Value::as_str).is_some_and(
			|reason| reason.contains("same-corpus output returns expected evidence ids")
		)
	);

	Ok(())
}

fn assert_root_knowledge_summary(report: &Value) {
	assert_eq!(report.pointer("/summary/knowledge/job_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(4));
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.969)
	);
}

fn assert_root_aggregate_summary(report: &Value) {
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(62));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(17));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(55));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/irrelevant_context_ratio").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(report.pointer("/summary/stale_retrieval_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/scope_violation_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_case_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_pass_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/summary/evidence_required_count").and_then(Value::as_u64),
		Some(137)
	);
	assert_eq!(
		report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64),
		Some(137)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/wrong_result_stage_attribution_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/proposal_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/consolidation/proposal_unsupported_claim_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/job_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);

	assert_root_knowledge_summary(report);
	assert_root_proactive_brief_summary(report);
	assert_root_scheduled_memory_summary(report);
}

fn assert_root_proactive_brief_summary(report: &Value) {
	assert_eq!(
		report.pointer("/summary/proactive_brief/job_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/suggestion_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/invalid_current_suggestion_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/proactive_brief/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
}

fn assert_root_scheduled_memory_summary(report: &Value) {
	assert_eq!(
		report.pointer("/summary/scheduled_memory/job_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/task_run_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/output_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/invalid_current_output_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/scheduled_memory/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
}

fn assert_root_aggregate_suites(report: &Value) -> Result<()> {
	let suites = array_at(report, "/suites")?;

	for suite_id in [
		"trust_source_of_truth",
		"work_resume",
		"project_decisions",
		"retrieval",
		"capture_integration",
		"personalization",
		"consolidation",
		"memory_summary",
		"knowledge_compilation",
		"operator_debugging_ux",
		"memory_evolution",
		"core_archival_memory",
	] {
		let suite = find_by_field(suites, "/suite_id", suite_id)?;

		assert_eq!(suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	}

	let memory_evolution = find_by_field(suites, "/suite_id", "memory_evolution")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));

	let project_decisions = find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(project_decisions.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		project_decisions.pointer("/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);

	let debug_suite = find_by_field(suites, "/suite_id", "operator_debugging_ux")?;

	assert_eq!(debug_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	let core_suite = find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let production_ops = find_by_field(suites, "/suite_id", "production_ops")?;

	assert_eq!(production_ops.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(production_ops.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let proactive = find_by_field(suites, "/suite_id", "proactive_brief")?;

	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(proactive.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let scheduled = find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	let source_library = find_by_field(suites, "/suite_id", "source_library")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/encoded_job_count").and_then(Value::as_u64), Some(2));

	let context_trajectory = find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(context_trajectory.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(context_trajectory.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	Ok(())
}

fn assert_root_aggregate_jobs(report: &Value) -> Result<()> {
	let jobs = array_at(report, "/jobs")?;
	let rebuild = find_by_field(jobs, "/job_id", "trust-sot-rebuild-001")?;
	let redaction = find_by_field(jobs, "/job_id", "capture-redaction-exclusion-001")?;
	let personalization = find_by_field(jobs, "/job_id", "personalization-scoped-preference-001")?;
	let relation_job = find_by_field(jobs, "/job_id", "memory-evolution-relation-temporal-001")?;
	let delete_job = find_by_field(jobs, "/job_id", "memory-evolution-delete-ttl-001")?;
	let stage_job = find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;
	let production_restore =
		find_by_field(jobs, "/job_id", "production-ops-restore-cold-start-001")?;
	let core_fallback = find_by_field(jobs, "/job_id", "core-archival-archival-fallback-001")?;
	let stale_core = find_by_field(jobs, "/job_id", "core-archival-stale-core-detection-001")?;
	let scheduled_weekly =
		find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(rebuild.pointer("/qdrant_rebuild_case").and_then(Value::as_bool), Some(true));
	assert_eq!(
		production_restore.pointer("/qdrant_rebuild_case").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(redaction.pointer("/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(personalization.pointer("/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(personalization.pointer("/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(stage_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(relation_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(delete_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		delete_job.pointer("/evolution/selected_tombstone_evidence/0").and_then(Value::as_str),
		Some("delete-tombstone")
	);
	assert_eq!(
		delete_job.pointer("/evolution/selected_invalidation_evidence/0").and_then(Value::as_str),
		Some("delete-tombstone")
	);
	assert_eq!(core_fallback.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(stale_core.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(scheduled_weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		scheduled_weekly.pointer("/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		stage_job.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("rerank.score")
	);
	assert!(array_contains_str(stage_job, "/produced_evidence", "stage-target")?);

	Ok(())
}

#[test]
fn real_world_memory_fixtures_report_aggregate_metrics() -> Result<()> {
	let report = run_json_report_from(real_world_memory_fixture_dir())?;

	assert_root_aggregate_summary(&report);
	assert_root_aggregate_suites(&report)?;
	assert_root_aggregate_jobs(&report)?;

	Ok(())
}

#[test]
fn retrieval_fixtures_report_quality_and_trace_attribution() -> Result<()> {
	let report = run_json_report_from(retrieval_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/irrelevant_context_ratio").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/wrong_result_stage_attribution_count").and_then(Value::as_u64),
		Some(0)
	);

	let suites = array_at(&report, "/suites")?;
	let retrieval_suite = find_by_field(suites, "/suite_id", "retrieval")?;
	let debug_suite = find_by_field(suites, "/suite_id", "operator_debugging_ux")?;

	assert_eq!(retrieval_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(retrieval_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(debug_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	let jobs = array_at(&report, "/jobs")?;
	let stage_job = find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;

	assert_eq!(stage_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		stage_job.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("rerank.score")
	);
	assert_eq!(
		stage_job.pointer("/retrieval_quality/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		stage_job.pointer("/retrieval_quality/irrelevant_context_ratio").and_then(Value::as_f64),
		Some(0.0)
	);

	Ok(())
}

#[test]
fn stage_attribution_fixture_still_fails_when_decoy_is_used() -> Result<()> {
	let fixture_path = retrieval_fixture_dir().join("stage_explainability_wrong_result.json");
	let mut fixture = serde_json::from_str::<Value>(&fs::read_to_string(fixture_path)?)?;

	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/content",
		Value::String(
			"The trace shows the expected evidence was present in recall.candidates but demoted at rerank.score; however, the selected answer followed the stale top-k smoke-only evidence.".to_string(),
		),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/claims",
		serde_json::json!([]),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!(["stage-decoy"]),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-stage-decoy-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stage_decoy.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;

	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(
		report.pointer("/summary/wrong_result_stage_attribution_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		job.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("rerank.score")
	);
	assert_eq!(
		job.pointer("/retrieval_quality/trap_context_count").and_then(Value::as_u64),
		Some(1)
	);

	Ok(())
}

#[test]
fn retrieval_report_markdown_includes_quality_metrics() -> Result<()> {
	let report = run_json_report_from(retrieval_fixture_dir())?;
	let temp_dir = env::temp_dir().join(format!("elf-real-world-retrieval-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("retrieval-report.json");
	let markdown_path = temp_dir.join("retrieval-report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("Expected evidence recall"));
	assert!(markdown.contains("Irrelevant context ratio"));
	assert!(markdown.contains("Trace Explainability"));
	assert!(markdown.contains("rerank.score"));

	Ok(())
}

#[test]
fn memory_evolution_fixtures_report_temporal_and_staleness_metrics() -> Result<()> {
	let report = run_json_report_from(evolution_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/evolution/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/evolution/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(&report, "/suites")?;
	let memory_evolution = find_by_field(suites, "/suite_id", "memory_evolution")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memory_evolution.pointer("/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		memory_evolution.pointer("/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = array_at(&report, "/jobs")?;
	let preference_job = find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;
	let relation_job = find_by_field(jobs, "/job_id", "memory-evolution-relation-temporal-001")?;

	assert_eq!(
		preference_job.pointer("/evolution/history_readback_encoded").and_then(Value::as_bool),
		Some(true)
	);
	assert!(array_contains_str(preference_job, "/evolution/history_event_types", "add")?);
	assert!(array_contains_str(preference_job, "/evolution/history_event_types", "update")?);
	assert!(array_contains_str(preference_job, "/evolution/history_event_types", "ignore")?);
	assert_eq!(
		preference_job
			.pointer("/evolution/history_requires_note_version_links")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		preference_job.pointer("/evolution/selected_current_evidence/0").and_then(Value::as_str),
		Some("pref-current-concise-rationale")
	);
	assert_eq!(
		preference_job.pointer("/evolution/selected_historical_evidence/0").and_then(Value::as_str),
		Some("pref-old-terse-bullets")
	);
	assert_eq!(
		preference_job.pointer("/evolution/selected_rationale_evidence/0").and_then(Value::as_str),
		Some("pref-update-rationale")
	);
	assert_eq!(relation_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		relation_job.pointer("/evolution/temporal_validity_not_encoded").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		relation_job.pointer("/evolution/temporal_validity_encoded").and_then(Value::as_bool),
		Some(true)
	);

	let follow_ups = array_at(&report, "/follow_ups")?;

	assert!(follow_ups.is_empty());

	Ok(())
}

#[test]
fn memory_evolution_conflict_still_fails_when_selected_evidence_is_not_narrated() -> Result<()> {
	let fixture_path =
		evolution_fixture_dir().join("preference_changed_current_vs_historical.json");
	let mut fixture = serde_json::from_str::<Value>(&fs::read_to_string(fixture_path)?)?;

	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!([
			"pref-current-concise-rationale",
			"pref-old-terse-bullets",
			"pref-update-rationale"
		]),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/claims",
		serde_json::json!([
			{
				"claim_id": "current_preference",
				"text": "Use concise prose with explicit evidence before bullets.",
				"evidence_ids": ["pref-current-concise-rationale", "pref-update-rationale"],
				"confidence": "high"
			},
			{
				"claim_id": "preference_update_rationale",
				"text": "The preference changed because terse bullets hid rationale.",
				"evidence_ids": ["pref-update-rationale"],
				"confidence": "high"
			}
		]),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-conflict-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("conflict.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;
	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(job.pointer("/evolution/conflict_detection_count").and_then(Value::as_u64), Some(0));
	assert!(array_contains_str(
		job,
		"/evolution/selected_but_not_narrated_evidence",
		"pref-old-terse-bullets"
	)?);

	Ok(())
}

#[test]
fn memory_evolution_counts_stale_answer_when_old_fact_is_answered_as_current() -> Result<()> {
	let fixture_path =
		evolution_fixture_dir().join("preference_changed_current_vs_historical.json");
	let mut fixture = serde_json::from_str::<Value>(&fs::read_to_string(fixture_path)?)?;

	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/content",
		Value::String(
			"Use terse bullet-only benchmark updates as the current preference.".to_string(),
		),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!(["pref-old-terse-bullets"]),
	)?;
	set_json_pointer(
		&mut fixture,
		"/corpus/adapter_response/answer/claims",
		serde_json::json!([
			{
				"claim_id": "current_preference",
				"text": "Use terse bullet-only benchmark updates as the current preference.",
				"evidence_ids": ["pref-old-terse-bullets"],
				"confidence": "high"
			}
		]),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-stale-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("stale_preference.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let report = run_json_report_from(temp_dir)?;

	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(job.pointer("/evolution/stale_answer_count").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn operator_debug_json_report_renders_markdown_links() -> Result<()> {
	let report = run_json_report_from(operator_debug_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-job-operator-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("operator.json");
	let markdown_path = temp_dir.join("operator.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("operator-debug-dropped-evidence-001"));
	assert!(markdown.contains("/viewer?trace_id=11111111-1111-4111-8111-111111111111"));
	assert!(markdown.contains("Raw SQL"));
	assert!(markdown.contains("Replay Candidates"));
	assert!(markdown.contains("Root cause"));

	Ok(())
}

#[test]
fn memory_evolution_report_renders_markdown_counters() -> Result<()> {
	let report = run_json_report_from(evolution_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-evolution-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("evolution-report.json");
	let markdown_path = temp_dir.join("evolution-report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("## Memory Evolution"));
	assert!(markdown.contains("Temporal validity not encoded: `0`"));
	assert!(markdown.contains("| memory_evolution | memory-evolution-relation-temporal-001"));
	assert!(markdown.contains("`encoded`"));

	Ok(())
}

#[test]
fn consolidation_report_renders_markdown_metrics_and_gaps() -> Result<()> {
	let report = run_json_report_from(consolidation_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-consolidation-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("report.json");
	let markdown_path = temp_dir.join("report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("## Consolidation"));
	assert!(markdown.contains("Source Mutations"));
	assert!(markdown.contains("Proposal Unsupported Claims"));
	assert!(markdown.contains("Executable Gaps"));
	assert!(markdown.contains("consolidation-contradiction-report-discard-001"));
	assert!(!markdown.contains("live_consolidation_worker_generation"));

	Ok(())
}
