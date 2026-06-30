mod graph_rag;
mod loss_summary;
mod manifest_summary;
mod validation;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn real_world_report_includes_external_adapter_coverage_manifest() -> Result<()> {
	let report = support::run_json_report_from(support::real_world_memory_fixture_dir())?;

	manifest_summary::assert_external_adapter_manifest_summary(&report);

	assert_external_adapter_manifest_records(&report)?;

	Ok(())
}

fn assert_external_adapter_manifest_records(report: &Value) -> Result<()> {
	let adapters = support::array_at(report, "/external_adapters/adapters")?;
	let elf = support::find_by_field(adapters, "/adapter_id", "elf_real_world_memory_fixture")?;
	let elf_live = support::find_by_field(adapters, "/adapter_id", "elf_live_real_world")?;
	let elf_operator_debug =
		support::find_by_field(adapters, "/adapter_id", "elf_operator_debug_live")?;
	let qmd = support::find_by_field(adapters, "/adapter_id", "qmd_live_baseline")?;
	let qmd_live = support::find_by_field(adapters, "/adapter_id", "qmd_live_real_world")?;
	let qmd_operator_debug =
		support::find_by_field(adapters, "/adapter_id", "qmd_operator_debug_live")?;
	let agentmemory = support::find_by_field(adapters, "/adapter_id", "agentmemory_live_baseline")?;
	let mem0 = support::find_by_field(adapters, "/adapter_id", "mem0_openmemory_live_baseline")?;
	let memsearch = support::find_by_field(adapters, "/adapter_id", "memsearch_live_baseline")?;
	let openviking = support::find_by_field(adapters, "/adapter_id", "openviking_live_baseline")?;
	let claude_mem = support::find_by_field(adapters, "/adapter_id", "claude_mem_live_baseline")?;
	let ragflow = support::find_by_field(adapters, "/adapter_id", "ragflow_research_gate")?;
	let lightrag = support::find_by_field(adapters, "/adapter_id", "lightrag_research_gate")?;
	let graphrag = support::find_by_field(adapters, "/adapter_id", "graphrag_research_gate")?;
	let graphiti_zep =
		support::find_by_field(adapters, "/adapter_id", "graphiti_zep_research_gate")?;
	let graphify = support::find_by_field(adapters, "/adapter_id", "graphify_docker_smoke")?;
	let qmd_deep = support::find_by_field(adapters, "/adapter_id", "qmd_deep_profile_gate")?;
	let openviking_deep =
		support::find_by_field(adapters, "/adapter_id", "openviking_deep_profile_gate")?;
	let letta = support::find_by_field(adapters, "/adapter_id", "letta_research_gate")?;

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

	graph_rag::assert_graphify_adapter(graphify)?;
	graph_rag::assert_graph_rag_representative_scenarios(
		ragflow,
		lightrag,
		graphrag,
		graphiti_zep,
		graphify,
	)?;

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

	let suites = support::array_at(adapter, "/suites")?;
	let core_suite = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core_suite.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("real_world_job_adapter")
	);
	assert_eq!(adapter.pointer("/capabilities/2/status").and_then(Value::as_str), Some("blocked"));

	let scenarios = support::array_at(adapter, "/scenarios")?;
	let attachment =
		support::find_by_field(scenarios, "/scenario_id", "core_block_attachment_readback")?;
	let scope = support::find_by_field(scenarios, "/scenario_id", "core_block_scope_readback")?;
	let provenance =
		support::find_by_field(scenarios, "/scenario_id", "core_block_provenance_readback")?;
	let stale = support::find_by_field(scenarios, "/scenario_id", "stale_core_detection")?;
	let fallback = support::find_by_field(scenarios, "/scenario_id", "archival_fallback_readback")?;
	let decision = support::find_by_field(
		scenarios,
		"/scenario_id",
		"core_archival_project_decision_recovery",
	)?;

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

	let suites = support::array_at(adapter, "/suites")?;
	let core_archival = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;
	let scheduled = support::find_by_field(suites, "/suite_id", "scheduled_memory")?;
	let context_trajectory = support::find_by_field(suites, "/suite_id", "context_trajectory")?;

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

	let elf_scenarios = support::array_at(elf, "/scenarios")?;
	let elf_trace =
		support::find_by_field(elf_scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let elf_replay =
		support::find_by_field(elf_scenarios, "/scenario_id", "operator_debug_replay_command")?;
	let elf_candidate = support::find_by_field(
		elf_scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let elf_repair = support::find_by_field(
		elf_scenarios,
		"/scenario_id",
		"operator_debug_repair_action_clarity",
	)?;
	let elf_selected = support::find_by_field(
		elf_scenarios,
		"/scenario_id",
		"operator_debug_selected_but_not_narrated",
	)?;

	assert_eq!(elf_scenarios.len(), 5);
	assert_eq!(elf_trace.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(elf_trace.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(elf_replay.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(elf_candidate.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(elf_repair.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(elf_selected.pointer("/comparison_outcome").and_then(Value::as_str), Some("win"));

	assert_operator_debug_qmd_adapter_record(qmd)?;

	assert!(support::array_at(elf, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("narrow operator-debug live slice"))
	}));
	assert!(support::array_at(qmd, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("narrow operator-debug live slice"))
	}));

	Ok(())
}

fn assert_operator_debug_qmd_adapter_record(qmd: &Value) -> Result<()> {
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

	let qmd_scenarios = support::array_at(qmd, "/scenarios")?;
	let qmd_trace =
		support::find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let qmd_replay =
		support::find_by_field(qmd_scenarios, "/scenario_id", "operator_debug_replay_command")?;
	let qmd_candidate = support::find_by_field(
		qmd_scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let qmd_repair = support::find_by_field(
		qmd_scenarios,
		"/scenario_id",
		"operator_debug_repair_action_clarity",
	)?;
	let qmd_selected = support::find_by_field(
		qmd_scenarios,
		"/scenario_id",
		"operator_debug_selected_but_not_narrated",
	)?;

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
fn assert_live_sweep_record(adapter: &Value, production_ops_status: &str) -> Result<()> {
	let suites = support::array_at(adapter, "/suites")?;
	let capabilities = support::array_at(adapter, "/capabilities")?;
	let adapter_id = adapter.pointer("/adapter_id").and_then(Value::as_str).unwrap_or_default();
	let targeted = support::find_by_field(capabilities, "/capability", "targeted_live_pass")?;
	let full_pass = support::find_by_field(capabilities, "/capability", "full_suite_live_pass")?;
	let work_resume = support::find_by_field(suites, "/suite_id", "work_resume")?;
	let memory_evolution = support::find_by_field(suites, "/suite_id", "memory_evolution")?;
	let production_ops = support::find_by_field(suites, "/suite_id", "production_ops")?;
	let consolidation = support::find_by_field(suites, "/suite_id", "consolidation")?;
	let knowledge = support::find_by_field(suites, "/suite_id", "knowledge_compilation")?;
	let operator_debug = support::find_by_field(suites, "/suite_id", "operator_debugging_ux")?;
	let capture = support::find_by_field(suites, "/suite_id", "capture_integration")?;
	let personalization = support::find_by_field(suites, "/suite_id", "personalization")?;
	let core_archival = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;
	let context_trajectory = support::find_by_field(suites, "/suite_id", "context_trajectory")?;
	let trust_sot = support::find_by_field(suites, "/suite_id", "trust_source_of_truth")?;
	let retrieval = support::find_by_field(suites, "/suite_id", "retrieval")?;
	let project_decisions = support::find_by_field(suites, "/suite_id", "project_decisions")?;

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
