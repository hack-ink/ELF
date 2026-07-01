mod external_adapters_first_generation;
mod external_adapters_fixture;
mod external_adapters_graph_gates;
mod external_adapters_letta;
mod external_adapters_live_sweep;
mod external_adapters_operator_debug;
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

	external_adapters_fixture::assert_elf_fixture_adapter_record(elf)?;

	assert_eq!(
		elf_live.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(elf_live.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));

	external_adapters_live_sweep::assert_live_sweep_record(elf_live, "blocked")?;
	external_adapters_operator_debug::assert_operator_debug_live_adapter_records(
		elf_operator_debug,
		qmd_operator_debug,
	)?;

	assert_eq!(qmd.pointer("/overall_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(qmd.pointer("/suites/0/status").and_then(Value::as_str), Some("not_encoded"));

	external_adapters_fixture::assert_qmd_live_baseline_record(qmd);

	assert_eq!(
		qmd_live.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(qmd_live.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));

	external_adapters_live_sweep::assert_live_sweep_record(qmd_live, "blocked")?;

	assert_eq!(
		agentmemory.pointer("/capabilities/1/status").and_then(Value::as_str),
		Some("mocked")
	);

	external_adapters_first_generation::assert_first_generation_adapter_records(
		agentmemory,
		mem0,
		memsearch,
		claude_mem,
	);

	assert_eq!(openviking.pointer("/overall_status").and_then(Value::as_str), Some("wrong_result"));

	external_adapters_graph_gates::assert_graph_rag_research_gate_records(
		ragflow, lightrag, graphrag,
	);
	external_adapters_graph_gates::assert_graphiti_zep_adapter(graphiti_zep);
	graph_rag::assert_graphify_adapter(graphify)?;
	graph_rag::assert_graph_rag_representative_scenarios(
		ragflow,
		lightrag,
		graphrag,
		graphiti_zep,
		graphify,
	)?;
	external_adapters_letta::assert_letta_core_archival_gate(letta)?;
	external_adapters_fixture::assert_qmd_deep_profile_gate(qmd_deep);

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

	external_adapters_fixture::assert_openviking_deep_profile_gate(openviking_deep);

	assert_eq!(
		openviking_deep.pointer("/result/artifact").and_then(Value::as_str),
		Some("docs/evidence/benchmarking/2026-06-11-qmd-openviking-strength-profile-report.md")
	);

	Ok(())
}
