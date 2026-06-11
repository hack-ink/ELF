#![allow(unused_crate_dependencies)]

//! Integration tests for the real-world job smoke benchmark runner.

use std::{
	env, fs,
	path::{Path, PathBuf},
	process::{self, Command},
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

fn consolidation_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("consolidation")
}

fn knowledge_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("knowledge")
}

fn production_ops_fixture_dir() -> PathBuf {
	real_world_memory_fixture_dir().join("production_ops")
}

fn workspace_root() -> Result<PathBuf> {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let root = manifest_dir
		.parent()
		.and_then(Path::parent)
		.ok_or_else(|| eyre::eyre!("could not resolve workspace root"))?;

	Ok(root.to_path_buf())
}

fn strength_profile_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("research")
		.join("2026-06-11-qmd-openviking-strength-profile-report.json"))
}

fn strength_profile_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("guide")
		.join("benchmarking")
		.join("2026-06-11-qmd-openviking-strength-profile-report.md"))
}

fn measurement_coverage_audit_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("guide")
		.join("benchmarking")
		.join("2026-06-11-measurement-coverage-audit.md"))
}

fn measurement_coverage_audit_json_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("research")
		.join("2026-06-11-measurement-coverage-audit.json"))
}

fn retrieval_debug_profile_json_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("research")
		.join("2026-06-11-elf-qmd-retrieval-debug-profile.json"))
}

fn competitor_strength_matrix_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("guide")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-evidence-matrix.md"))
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

fn set_json_pointer(value: &mut Value, pointer: &str, replacement: Value) -> Result<()> {
	let target =
		value.pointer_mut(pointer).ok_or_else(|| eyre::eyre!("missing JSON pointer {pointer}"))?;

	*target = replacement;

	Ok(())
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
		Some(21)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/live_real_world_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(12)
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

fn assert_external_adapter_manifest_summary(report: &Value) {
	assert_eq!(
		report.pointer("/external_adapters/schema").and_then(Value::as_str),
		Some("elf.real_world_external_adapter_report/v1")
	);
	assert_eq!(
		report.pointer("/external_adapters/manifest_id").and_then(Value::as_str),
		Some("real-world-memory-project-adapters-2026-06-11")
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
		Some(21)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/external_project_count").and_then(Value::as_u64),
		Some(19)
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
		Some(2)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(12)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/pass")
			.and_then(Value::as_u64),
		Some(1)
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
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(7)
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
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/suite_status_counts/blocked")
			.and_then(Value::as_u64),
		Some(13)
	);
}

fn assert_external_adapter_manifest_records(report: &Value) -> Result<()> {
	let adapters = array_at(report, "/external_adapters/adapters")?;
	let elf = find_by_field(adapters, "/adapter_id", "elf_real_world_memory_fixture")?;
	let elf_live = find_by_field(adapters, "/adapter_id", "elf_live_real_world")?;
	let qmd = find_by_field(adapters, "/adapter_id", "qmd_live_baseline")?;
	let qmd_live = find_by_field(adapters, "/adapter_id", "qmd_live_real_world")?;
	let agentmemory = find_by_field(adapters, "/adapter_id", "agentmemory_live_baseline")?;
	let mem0 = find_by_field(adapters, "/adapter_id", "mem0_openmemory_live_baseline")?;
	let memsearch = find_by_field(adapters, "/adapter_id", "memsearch_live_baseline")?;
	let openviking = find_by_field(adapters, "/adapter_id", "openviking_live_baseline")?;
	let claude_mem = find_by_field(adapters, "/adapter_id", "claude_mem_live_baseline")?;
	let ragflow = find_by_field(adapters, "/adapter_id", "ragflow_research_gate")?;
	let lightrag = find_by_field(adapters, "/adapter_id", "lightrag_research_gate")?;
	let graphrag = find_by_field(adapters, "/adapter_id", "graphrag_research_gate")?;
	let graphiti_zep = find_by_field(adapters, "/adapter_id", "graphiti_zep_research_gate")?;
	let graphify = find_by_field(adapters, "/adapter_id", "graphify_research_gate")?;
	let qmd_deep = find_by_field(adapters, "/adapter_id", "qmd_deep_profile_gate")?;
	let openviking_deep = find_by_field(adapters, "/adapter_id", "openviking_deep_profile_gate")?;

	assert_eq!(elf.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
	assert_eq!(elf.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		elf_live.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(elf_live.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));

	assert_live_sweep_record(elf_live, "blocked")?;

	assert_eq!(qmd.pointer("/overall_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(qmd.pointer("/suites/0/status").and_then(Value::as_str), Some("not_encoded"));
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

	assert_first_generation_adapter_records(mem0, memsearch, claude_mem);

	assert_eq!(openviking.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(ragflow.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(ragflow.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		ragflow.pointer("/execution_metadata/research_depth").and_then(Value::as_str),
		Some(
			"D2 feasibility verdict plus XY-885 evidence-smoke implementation; checked-in record remains research_gate unless a generated artifact reaches query output"
		)
	);
	assert_eq!(
		ragflow.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make ragflow-docker-smoke")
	);
	assert_eq!(
		ragflow.pointer("/result/artifact").and_then(Value::as_str),
		Some("tmp/real-world-memory/ragflow-smoke/ragflow-smoke.json")
	);
	assert_eq!(
		ragflow.pointer("/execution_metadata/sources/0/url").and_then(Value::as_str),
		Some("https://github.com/infiniflow/ragflow")
	);
	assert_eq!(lightrag.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(lightrag.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		lightrag.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make lightrag-docker-context-smoke")
	);
	assert_eq!(
		lightrag.pointer("/run/command").and_then(Value::as_str),
		Some("ELF_LIGHTRAG_CONTEXT_START=1 cargo make lightrag-docker-context-smoke")
	);
	assert_eq!(
		lightrag.pointer("/capabilities/3/status").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(graphrag.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(
		graphrag.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make graphrag-docker-smoke")
	);
	assert_eq!(graphrag.pointer("/suites/1/status").and_then(Value::as_str), Some("not_encoded"));

	assert_graphiti_zep_adapter(graphiti_zep);
	assert_graphify_adapter(graphify);

	assert_eq!(
		qmd_deep.pointer("/capabilities/2/status").and_then(Value::as_str),
		Some("unsupported")
	);
	assert_eq!(
		qmd_deep.pointer("/result/artifact").and_then(Value::as_str),
		Some("docs/research/2026-06-11-qmd-openviking-strength-profile-report.json")
	);
	assert_eq!(
		openviking_deep.pointer("/adapter_kind").and_then(Value::as_str),
		Some("docker_local_embed_context_trajectory_gate")
	);
	assert_eq!(
		openviking_deep.pointer("/result/artifact").and_then(Value::as_str),
		Some("docs/research/2026-06-11-qmd-openviking-strength-profile-report.json")
	);

	Ok(())
}

fn assert_first_generation_adapter_records(mem0: &Value, memsearch: &Value, claude_mem: &Value) {
	assert_eq!(
		mem0.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("local_lifecycle_update_delete_reload")
	);
	assert_eq!(mem0.pointer("/capabilities/2/status").and_then(Value::as_str), Some("real"));
	assert_eq!(mem0.pointer("/capabilities/4/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(
		memsearch.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("reindex_update_delete_reload")
	);
	assert_eq!(memsearch.pointer("/capabilities/2/status").and_then(Value::as_str), Some("real"));
	assert_eq!(claude_mem.pointer("/capabilities/1/status").and_then(Value::as_str), Some("real"));
	assert_eq!(
		claude_mem.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("repository_progressive_disclosure")
	);
	assert_eq!(
		claude_mem.pointer("/capabilities/4/status").and_then(Value::as_str),
		Some("not_encoded")
	);
}

fn assert_graphiti_zep_adapter(adapter: &Value) {
	assert_eq!(adapter.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make graphiti-zep-docker-temporal-smoke")
	);
	assert_eq!(
		adapter.pointer("/run/command").and_then(Value::as_str),
		Some(
			"ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make graphiti-zep-docker-temporal-smoke"
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
			"D2 feasibility plus XY-888 Docker temporal smoke implementation; checked-in record remains research_gate unless a generated artifact reaches Graphiti search output"
		)
	);
}

fn assert_graphify_adapter(adapter: &Value) {
	assert_eq!(adapter.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make graphify-docker-graph-report-smoke")
	);
	assert_eq!(
		adapter.pointer("/suites/0/suite_id").and_then(Value::as_str),
		Some("knowledge_compilation")
	);
	assert_eq!(adapter.pointer("/suites/0/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(adapter.pointer("/suites/1/suite_id").and_then(Value::as_str), Some("retrieval"));
	assert_eq!(adapter.pointer("/suites/1/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/execution_metadata/research_depth").and_then(Value::as_str),
		Some(
			"D1 feasibility verdict plus XY-889 Docker graph/report smoke implementation; checked-in record remains research_gate unless a generated artifact reaches graphify output"
		)
	);
}

fn assert_live_sweep_record(adapter: &Value, production_ops_status: &str) -> Result<()> {
	let suites = array_at(adapter, "/suites")?;
	let capabilities = array_at(adapter, "/capabilities")?;
	let targeted = find_by_field(capabilities, "/capability", "targeted_live_pass")?;
	let full_pass = find_by_field(capabilities, "/capability", "full_suite_live_pass")?;
	let work_resume = find_by_field(suites, "/suite_id", "work_resume")?;
	let memory_evolution = find_by_field(suites, "/suite_id", "memory_evolution")?;
	let production_ops = find_by_field(suites, "/suite_id", "production_ops")?;
	let consolidation = find_by_field(suites, "/suite_id", "consolidation")?;

	assert_eq!(targeted.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(full_pass.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(work_resume.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		production_ops.pointer("/status").and_then(Value::as_str),
		Some(production_ops_status)
	);
	assert_eq!(consolidation.pointer("/status").and_then(Value::as_str), Some("not_encoded"));

	Ok(())
}

#[test]
fn runner_discovers_nested_fixture_layout() -> Result<()> {
	let report = run_json_report_from(fixture_root())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(38));

	Ok(())
}

#[test]
fn operator_debug_fixture_reports_trace_links_and_failure_details() -> Result<()> {
	let report = run_json_report_from(operator_debug_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		report.pointer("/summary/operator_debug_job_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(report.pointer("/summary/raw_sql_needed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/trace_incomplete_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/operator_ux_gap_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = array_at(&report, "/jobs")?;
	let dropped = find_by_field(jobs, "/job_id", "operator-debug-dropped-evidence-001")?;

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

	assert_strength_profile_summary(&report);
	assert_qmd_strength_profile(&report)?;
	assert_qmd_wrong_result_diagnosis(&report)?;
	assert_openviking_strength_profile(&report)?;
	assert_strength_profile_markdown_boundaries(&markdown);

	Ok(())
}

#[test]
fn current_benchmark_reports_preserve_live_sweep_boundaries() -> Result<()> {
	let measurement_audit = fs::read_to_string(measurement_coverage_audit_path()?)?;
	let measurement_audit_json = serde_json::from_str::<Value>(&fs::read_to_string(
		measurement_coverage_audit_json_path()?,
	)?)?;
	let competitor_matrix = fs::read_to_string(competitor_strength_matrix_path()?)?;
	let external_manifest = fs::read_to_string(external_adapter_manifest_path())?;
	let retrieval_debug_profile =
		serde_json::from_str::<Value>(&fs::read_to_string(retrieval_debug_profile_json_path()?)?)?;

	assert!(
		measurement_audit.contains(
			"| `memory_evolution` | `6` | `pass:1`, `wrong_result:5` | `wrong_result:6` |"
		)
	);
	assert!(
		measurement_audit
			.contains("qmd live fails 6/6 jobs after missing the delete/TTL tombstone evidence")
	);
	assert!(
		competitor_matrix
			.contains("broader live suites remain `wrong_result`, `blocked`, or `not_encoded`")
	);
	assert!(external_manifest.contains(
		"The record is a full-suite sweep, not a full-suite pass; wrong_result, blocked, and not_encoded states remain visible."
	));

	for stale_phrase in [
		"same live sweep shape as ELF",
		"ELF and qmd live fail 5/6 jobs",
		"both systems currently fail 5/6 live memory-evolution jobs",
		"wrong_result, incomplete, blocked, and not_encoded states remain visible",
		"broader live suites remain `wrong_result`, `incomplete`, or `not_encoded`",
	] {
		assert!(!measurement_audit.contains(stale_phrase));
		assert!(!competitor_matrix.contains(stale_phrase));
		assert!(!external_manifest.contains(stale_phrase));
	}

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

	Ok(())
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
		report.pointer("/summary/qmd/debug_replay_ergonomics").and_then(Value::as_str),
		Some("elf_loss")
	);
	assert_eq!(
		report.pointer("/summary/openviking/overall_against_strengths").and_then(Value::as_str),
		Some("not_tested_on_context_trajectory")
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
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/elf_win")
			.and_then(Value::as_u64),
		Some(2)
	);
}

fn assert_qmd_strength_profile(report: &Value) -> Result<()> {
	let qmd_scenarios = array_at(report, "/qmd_strength_profile/scenario_outcomes")?;
	let local_transparency =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-local-query-transparency")?;
	let retrieval = find_by_field(qmd_scenarios, "/scenario_id", "qmd-retrieval-quality")?;
	let rerank_controls =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-expansion-fusion-rerank-controls")?;
	let wrong_result = find_by_field(qmd_scenarios, "/scenario_id", "qmd-wrong-result-diagnosis")?;

	assert_eq!(qmd_scenarios.len(), 8);
	assert_eq!(retrieval.pointer("/elf_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(
		local_transparency.pointer("/elf_outcome").and_then(Value::as_str),
		Some("elf_loss")
	);
	assert_eq!(
		rerank_controls.pointer("/result_type").and_then(Value::as_str),
		Some("not_encoded")
	);
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

	assert_eq!(openviking_scenarios.len(), 6);
	assert_eq!(
		trajectory.pointer("/evidence_class").and_then(Value::as_str),
		Some("research_gate")
	);
	assert_eq!(trajectory.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(precondition.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(precondition.pointer("/elf_outcome").and_then(Value::as_str), Some("elf_win"));
	assert_eq!(
		precondition.pointer("/typed_blocker").and_then(Value::as_str),
		Some("output_missed_expected_terms")
	);

	Ok(())
}

fn assert_strength_profile_markdown_boundaries(markdown: &str) {
	assert!(
		markdown.contains(
			"| Wrong-result diagnosis | `research_gate` | `not_encoded` | `not_tested` |"
		)
	);
	assert!(markdown.contains("no pass evidence is claimed"));
	assert!(markdown.contains("typed `wrong_result` state"));
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

	Ok(())
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

fn assert_root_knowledge_summary(report: &Value) {
	assert_eq!(report.pointer("/summary/knowledge/job_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(4));
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.969)
	);
}

fn assert_root_aggregate_summary(report: &Value) {
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(38));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(36));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
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
		Some(10)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(2));
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
		Some(84)
	);
	assert_eq!(report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64), Some(84));
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

	assert_root_knowledge_summary(report);
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
		"knowledge_compilation",
		"operator_debugging_ux",
		"memory_evolution",
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

	let production_ops = find_by_field(suites, "/suite_id", "production_ops")?;

	assert_eq!(production_ops.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(production_ops.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	Ok(())
}

fn assert_root_aggregate_jobs(report: &Value) -> Result<()> {
	let jobs = array_at(report, "/jobs")?;
	let rebuild = find_by_field(jobs, "/job_id", "trust-sot-rebuild-001")?;
	let redaction = find_by_field(jobs, "/job_id", "capture-redaction-exclusion-001")?;
	let personalization = find_by_field(jobs, "/job_id", "personalization-scoped-preference-001")?;
	let relation_job = find_by_field(jobs, "/job_id", "memory-evolution-relation-temporal-001")?;
	let stage_job = find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;
	let production_restore =
		find_by_field(jobs, "/job_id", "production-ops-restore-cold-start-001")?;

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
