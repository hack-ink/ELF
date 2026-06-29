use std::{
	env, fs,
	path::Path,
	process::{self, Command},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

use super::support::*;

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
		Some(52)
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
		Some(18)
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
		Some(26)
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
		Some(5)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(14)
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
		Some(10)
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
		Some(29)
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
		Some(24)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_status_counts/incomplete")
			.and_then(Value::as_u64),
		Some(5)
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
		Some(13)
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
		Some(53)
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
		Some(19)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/blocked")
			.and_then(Value::as_u64),
		Some(29)
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
		evidence.contains("82 jobs across 19 suites")
			&& evidence.contains("75 pass")
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

	assert_adapter_matrix_rows(
		ragflow_scenarios,
		&[
			("reference_chunk_citation_mapping", "blocked", "blocked"),
			("retrieval_quality_reference_recall", "blocked", "blocked"),
			("navigation_quality_document_chunks", "blocked", "blocked"),
			("answer_faithfulness_reference_chunks", "blocked", "blocked"),
			("stale_source_behavior", "not_encoded", "not_tested"),
			("knowledge_compilation_quality", "not_encoded", "not_tested"),
		],
	)?;
	assert_adapter_matrix_rows(
		lightrag_scenarios,
		&[
			("context_source_reference_mapping", "incomplete", "blocked"),
			("retrieval_quality_context_recall", "incomplete", "blocked"),
			("citation_quality_context_references", "incomplete", "blocked"),
			("navigation_quality_graph_context", "incomplete", "blocked"),
			("answer_faithfulness_context_refs", "incomplete", "blocked"),
			("stale_source_behavior", "not_encoded", "not_tested"),
			("knowledge_compilation_quality", "not_encoded", "not_tested"),
		],
	)?;
	assert_adapter_matrix_rows(
		graphrag_scenarios,
		&[
			("output_table_citation_mapping", "blocked", "blocked"),
			("retrieval_quality_local_search", "not_encoded", "not_tested"),
			("navigation_quality_community_graph", "blocked", "blocked"),
			("answer_faithfulness_output_tables", "blocked", "blocked"),
			("stale_source_behavior", "not_encoded", "not_tested"),
			("graph_summary_synthesis_quality", "not_encoded", "not_tested"),
		],
	)?;

	Ok(())
}

fn assert_adapter_matrix_rows(scenarios: &[Value], expected: &[(&str, &str, &str)]) -> Result<()> {
	for (scenario_id, status, outcome) in expected {
		let row = find_by_field(scenarios, "/scenario_id", scenario_id)?;

		assert_eq!(row.pointer("/status").and_then(Value::as_str), Some(*status));
		assert_eq!(row.pointer("/comparison_outcome").and_then(Value::as_str), Some(*outcome));
		assert!(
			row.pointer("/evidence")
				.and_then(Value::as_str)
				.is_some_and(|evidence| !evidence.trim().is_empty())
		);
	}

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
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
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
	assert_eq!(
		graphiti.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("graphiti.provider_boundary")
	);
	assert!(array_contains_str(graphiti, "/produced_evidence", "graphiti-current-fact-contract")?);
	assert!(array_contains_str(
		graphiti,
		"/produced_evidence",
		"graphiti-historical-fact-contract"
	)?);
	assert!(array_contains_str(graphiti, "/produced_evidence", "graphiti-provider-boundary")?);
	assert!(array_contains_str(graphify, "/produced_evidence", "graphify-source-location-output")?);

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
