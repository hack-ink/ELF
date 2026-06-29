use std::{fs, path::Path};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

fn real_world_live_adapter_sources(workspace: &Path) -> Result<String> {
	let mut source =
		fs::read_to_string(workspace.join("apps/elf-eval/src/bin/real_world_live_adapter.rs"))?;

	append_rust_sources(
		workspace.join("apps/elf-eval/src/bin/real_world_live_adapter").as_path(),
		&mut source,
	)?;

	Ok(source)
}

fn real_world_job_benchmark_sources(workspace: &Path) -> Result<String> {
	let mut source =
		fs::read_to_string(workspace.join("apps/elf-eval/src/bin/real_world_job_benchmark.rs"))?;

	append_rust_sources(
		workspace.join("apps/elf-eval/src/bin/real_world_job_benchmark").as_path(),
		&mut source,
	)?;

	Ok(source)
}

fn append_rust_sources(dir: &Path, source: &mut String) -> Result<()> {
	let mut entries = Vec::new();

	for entry in fs::read_dir(dir)? {
		entries.push(entry?.path());
	}

	entries.sort();

	for path in entries {
		if path.is_dir() {
			append_rust_sources(path.as_path(), source)?;
		} else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
			source.push('\n');
			source.push_str(fs::read_to_string(path)?.as_str());
		}
	}

	Ok(())
}

#[test]
fn live_adapter_aggregate_forwards_graph_rag_smoke_controls() -> Result<()> {
	let workspace = support::workspace_root()?;
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
	let workspace_root = support::workspace_root()?;
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
	let workspace = support::workspace_root()?;
	let makefile = fs::read_to_string(workspace.join("Makefile.toml"))?;
	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;
	let script = fs::read_to_string(
		workspace.join("scripts").join("real-world-operator-debug-live-adapters.sh"),
	)?;
	let live_adapter = real_world_live_adapter_sources(&workspace)?;
	let benchmark = real_world_job_benchmark_sources(&workspace)?;

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
fn live_adapter_supports_elf_capture_write_policy_without_external_hook_claims() -> Result<()> {
	let workspace = support::workspace_root()?;
	let live_adapter = real_world_live_adapter_sources(&workspace)?;
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
