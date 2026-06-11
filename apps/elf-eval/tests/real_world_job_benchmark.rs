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

fn trace_replay_diagnostics_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("research")
		.join("2026-06-11-elf-qmd-trace-replay-diagnostics-report.json"))
}

fn trace_replay_diagnostics_markdown_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("guide")
		.join("benchmarking")
		.join("2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"))
}

fn competitor_strength_adoption_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("guide")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-adoption-report.md"))
}

fn competitor_strength_adoption_report_json_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("research")
		.join("2026-06-11-competitor-strength-adoption-report.json"))
}

fn temporal_history_competitor_gap_json_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("research")
		.join("2026-06-11-temporal-history-competitor-gap-report.json"))
}

fn competitor_strength_matrix_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("guide")
		.join("benchmarking")
		.join("2026-06-11-competitor-strength-evidence-matrix.md"))
}

fn competitor_strength_matrix_json_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("research")
		.join("2026-06-11-xy-897-competitor-strength-matrix.json"))
}

fn readme_path() -> Result<PathBuf> {
	Ok(workspace_root()?.join("README.md"))
}

fn benchmarking_index_path() -> Result<PathBuf> {
	Ok(workspace_root()?.join("docs").join("guide").join("benchmarking").join("index.md"))
}

fn iteration_direction_report_path() -> Result<PathBuf> {
	Ok(workspace_root()?
		.join("docs")
		.join("guide")
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
		Some(3)
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
		Some(10)
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
		Some(7)
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
		Some("real-world-memory-project-adapters-2026-06-11-openmemory-ui-export")
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
		Some(3)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/pass")
			.and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/overall_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(5)
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
		Some(5)
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
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/suite_status_counts/blocked")
			.and_then(Value::as_u64),
		Some(13)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/suite_status_counts/incomplete")
			.and_then(Value::as_u64),
		Some(0)
	);

	assert_external_adapter_manifest_scenario_summary(report);
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
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/blocked")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/incomplete")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(1)
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
		Some(9)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/wins")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/ties")
			.and_then(Value::as_u64),
		Some(4)
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
		Some(11)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/win")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/tie")
			.and_then(Value::as_u64),
		Some(4)
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
		Some(8)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/blocked")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/non_goal")
			.and_then(Value::as_u64),
		Some(2)
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
	let graphify = find_by_field(adapters, "/adapter_id", "graphify_docker_smoke")?;
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
		Some("cargo make ragflow-docker-smoke")
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
	assert_graphify_adapter(graphify)?;

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

	assert_openviking_deep_profile_gate(openviking_deep);

	assert_eq!(
		openviking_deep.pointer("/result/artifact").and_then(Value::as_str),
		Some("docs/research/2026-06-11-qmd-openviking-strength-profile-report.json")
	);

	Ok(())
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

fn assert_openviking_deep_profile_gate(adapter: &Value) {
	let trajectory_evidence = adapter.pointer("/capabilities/1/evidence").and_then(Value::as_str);

	assert!(trajectory_evidence.is_some_and(|evidence| {
		evidence.contains("evidence-bearing same-corpus output")
			&& evidence.contains("wrong_result missed-term evidence")
			&& !evidence.contains("setup reaches runnable OpenViking APIs")
	}));
}

fn assert_first_generation_adapter_records(
	agentmemory: &Value,
	mem0: &Value,
	memsearch: &Value,
	claude_mem: &Value,
) {
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
	assert_eq!(
		memsearch.pointer("/scenarios/1/status").and_then(Value::as_str),
		Some("unsupported")
	);
	assert_eq!(
		memsearch.pointer("/scenarios/1/elf_position").and_then(Value::as_str),
		Some("untested")
	);
	assert_eq!(claude_mem.pointer("/capabilities/1/status").and_then(Value::as_str), Some("real"));
	assert_eq!(
		claude_mem.pointer("/capabilities/3/capability").and_then(Value::as_str),
		Some("repository_progressive_disclosure")
	);
	assert_eq!(
		claude_mem.pointer("/capabilities/4/status").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(
		claude_mem.pointer("/scenarios/0/status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(claude_mem.pointer("/scenarios/1/status").and_then(Value::as_str), Some("pass"));
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
		Some("cargo make graphify-docker-graph-report-smoke")
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
				"command": "cargo make graphify-docker-graph-report-smoke",
				"artifact": "tmp/real-world-memory/graphify-smoke/graphify-smoke.json"
			},
			"run": {
				"status": "pass",
				"evidence": "run evidence",
				"command": "cargo make graphify-docker-graph-report-smoke",
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
				"setup_path": "cargo make graphify-docker-graph-report-smoke",
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
fn live_adapter_aggregate_forwards_graph_rag_smoke_controls() -> Result<()> {
	let makefile = fs::read_to_string(
		Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("Makefile.toml"),
	)?;

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
			makefile.contains(&format!("-e {env_name}")),
			"real-world-memory-live-adapters must forward {env_name}",
		);
	}

	assert!(
		makefile.contains("--profile lightrag up -d lightrag"),
		"aggregate task should start LightRAG profile when ELF_LIGHTRAG_CONTEXT_START=1",
	);
	assert!(
		makefile.contains("--profile graphiti-zep up -d graphiti-falkordb"),
		"aggregate task should start Graphiti/Zep profile when ELF_GRAPHITI_ZEP_SMOKE_START=1",
	);

	Ok(())
}

#[test]
fn openmemory_ui_export_probe_has_dedicated_docker_task() -> Result<()> {
	let workspace_root = workspace_root()?;
	let makefile = fs::read_to_string(workspace_root.join("Makefile.toml"))?;
	let compose = fs::read_to_string(workspace_root.join("docker-compose.baseline.yml"))?;
	let script = fs::read_to_string(workspace_root.join("scripts/live-baseline-benchmark.sh"))?;
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		workspace_root.join("docs/research/2026-06-11-xy-931-openmemory-ui-export-readback.json"),
	)?)?;

	assert!(makefile.contains("[tasks.openmemory-ui-export-readback]"));
	assert!(makefile.contains("export ELF_BASELINE_PROJECTS=mem0"));
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

fn assert_live_sweep_record(adapter: &Value, production_ops_status: &str) -> Result<()> {
	let suites = array_at(adapter, "/suites")?;
	let capabilities = array_at(adapter, "/capabilities")?;
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
	let trust_sot = find_by_field(suites, "/suite_id", "trust_source_of_truth")?;
	let retrieval = find_by_field(suites, "/suite_id", "retrieval")?;
	let project_decisions = find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(suites.len(), 11);
	assert_eq!(targeted.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(full_pass.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert!(
		adapter
			.pointer("/result/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("38 jobs across all 11 encoded suites"))
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
	assert_eq!(consolidation.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(knowledge.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(operator_debug.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(personalization.pointer("/status").and_then(Value::as_str), Some("pass"));

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
	let external_manifest = fs::read_to_string(external_adapter_manifest_path())?;
	let retrieval_debug_profile =
		serde_json::from_str::<Value>(&fs::read_to_string(retrieval_debug_profile_json_path()?)?)?;
	let temporal_history = serde_json::from_str::<Value>(&fs::read_to_string(
		temporal_history_competitor_gap_json_path()?,
	)?)?;

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
	assert!(external_manifest.contains(
		"The qmd live real-world sweep covers the current encoded fixture corpus; expanded retrieval-debug strength suites still need their own materialized adapter run."
	));

	for stale_phrase in [
		"same live sweep shape as ELF",
		"ELF and qmd live fail 5/6 jobs",
		"both systems currently fail 5/6 live memory-evolution jobs",
		"wrong_result, incomplete, blocked, and not_encoded states remain visible",
		"broader live suites remain `wrong_result`, `incomplete`, or `not_encoded`",
		"The qmd live real-world slice covers representative jobs only",
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
	assert!(
		adoption_report
			.contains("Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF")
	);

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
	assert_eq!(report.pointer("/summary/outcome_counts/non_goal").and_then(Value::as_u64), Some(1));

	let scenarios = array_at(report, "/scenario_outcomes")?;
	let retrieval = find_by_field(scenarios, "/scenario_id", "retrieval_correctness_guardrail")?;
	let top10 = find_by_field(scenarios, "/scenario_id", "default_top10_candidate_artifact")?;
	let replay = find_by_field(scenarios, "/scenario_id", "replay_command_locality")?;
	let trace_surface =
		find_by_field(scenarios, "/scenario_id", "trace_admin_replay_surface_availability")?;
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

	assert_eq!(scenarios.len(), 11);
	assert_eq!(retrieval.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(top10.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(replay.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(trace_surface.pointer("/outcome").and_then(Value::as_str), Some("tie"));
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
		"Do not claim qmd beats ELF as a memory system overall."
	)?);

	Ok(())
}

fn assert_trace_replay_diagnostics_markdown(markdown: &str) {
	assert!(markdown.contains("Retrieval correctness is still tied"));
	assert!(markdown.contains("| Default top-10 candidate artifact |"));
	assert!(markdown.contains("| Replay command locality |"));
	assert!(markdown.contains("| Rerank attribution | `live_baseline_only` | `non_goal` |"));
	assert!(markdown.contains("| Candidate-drop diagnostics | `research_gate` | `not_encoded` |"));
	assert!(markdown.contains("`retrieved_but_dropped` | Defined but `not_tested`"));
	assert!(markdown.contains("npx tsx src/cli/qmd.ts query"));
	assert!(markdown.contains("cargo run -p elf-eval -- --config-a"));
	assert!(markdown.contains("Do not claim qmd beats ELF as a memory system overall"));
	assert!(markdown.contains("Do not score rerank superiority from a qmd `--no-rerank` run"));
}

fn assert_trace_replay_adoption_json(adoption: &Value) -> Result<()> {
	let local_debug = find_by_field(
		array_at(adoption, "/scenario_outcomes")?,
		"/scenario_id",
		"local_debug_replay_ux",
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
		"docs/guide/benchmarking/2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"
	)?);
	assert!(array_contains_str(
		adoption,
		"/claim_boundaries/not_allowed",
		"Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF memory-system or retrieval-quality win."
	)?);

	Ok(())
}

fn assert_competitor_strength_matrix_json(matrix: &Value) -> Result<()> {
	let projects = array_at(matrix, "/project_matrix")?;
	let qmd = find_by_field(projects, "/project", "qmd")?;
	let mem0 = find_by_field(projects, "/project", "mem0/OpenMemory")?;
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
		claim.contains("before claiming ELF wins, ties, or loses on retrieval debugging")
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
		Some("not_encoded")
	);
	assert!(
		openviking
			.pointer("/unsupported_or_blocked_status/details")
			.and_then(Value::as_str)
			.is_some_and(|details| details.contains("same-corpus output misses expected evidence"))
	);
	assert!(
		openviking
			.pointer("/benchmark_before_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("evidence-bearing same-corpus output pass"))
	);

	let scenarios = array_at(matrix, "/scenario_matrix")?;
	let retrieval_debug = find_by_field(scenarios, "/scenario_id", "retrieval_debug")?;
	let context_trajectory = find_by_field(scenarios, "/scenario_id", "context_trajectory")?;

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
		Some("research_gate")
	);
	assert_eq!(trajectory.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));
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
	assert_eq!(hierarchy.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(hierarchy.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(
		recursive_expansion.pointer("/result_type").and_then(Value::as_str),
		Some("not_encoded")
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
		"ELF does not beat OpenViking on context trajectory; OpenViking trajectory strengths remain not_tested behind a wrong_result same-corpus output precondition."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"Research_gate records are follow-up gates, not pass evidence."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"Missing equivalent surfaces are encoded as unsupported or not_encoded rather than fake losses."
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
		"Do not turn `research_gate`, `not_encoded`, or `unsupported` surfaces into wins"
	));
	assert!(markdown.contains("no pass evidence is claimed"));
	assert!(markdown.contains("typed `wrong_result` state"));
}

fn assert_operator_facing_strength_profile_boundaries(
	readme: &str,
	benchmarking_index: &str,
	iteration_direction: &str,
) {
	assert!(readme.contains("Full-suite live real-world adapter sweep after XY-899"));
	assert!(readme.contains("fresh ELF sweep reports 18 pass"));
	assert!(readme.contains("5 wrong_result, 2 blocked, and 13 not_encoded jobs"));
	assert!(readme.contains("fresh qmd sweep reports"));
	assert!(readme.contains("17 pass, 6 wrong_result, 2 blocked, and 13 not_encoded jobs"));
	assert!(readme.contains("The difference is the"));
	assert!(readme.contains("delete/TTL tombstone case"));
	assert!(readme.contains("qmd remains the local retrieval-debug UX reference"));
	assert!(readme.contains("no broad ELF-over-qmd claim is allowed"));
	assert!(readme.contains("qmd and OpenViking Strength-Profile Report - June 11, 2026"));
	assert!(benchmarking_index.contains("2026-06-11-qmd-openviking-strength-profile-report.md"));
	assert!(
		benchmarking_index.contains("separates qmd retrieval quality from debug/replay ergonomics")
	);
	assert!(benchmarking_index.contains("preserves OpenViking context-trajectory"));
	assert!(
		benchmarking_index
			.contains("surfaces as `not_tested` until staged/hierarchical evidence is encoded")
	);
	assert!(
		iteration_direction
			.contains("ELF and qmd are tied on the encoded live retrieval, work-resume, and")
	);
	assert!(iteration_direction.contains("ELF does not yet beat qmd's local retrieval-debug"));
	assert!(
		iteration_direction
			.contains("ELF beats OpenViking on context trajectory. That scenario is not encoded.")
	);
	assert!(
		iteration_direction
			.contains("Do not promote a reference project into a win/loss claim until")
	);
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
	assert!(markdown.contains("ELF scenario positions: `wins=2, ties=4, loses=1, untested=11`"));
	assert!(markdown.contains(
		"Scenario comparison outcomes: `win=2, tie=4, loss=1, not_tested=8, blocked=1, non_goal=2`"
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
