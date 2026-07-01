use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn agent_knowledge_os_closeout_benchmark_preserves_full_matrix_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::agent_knowledge_os_closeout_benchmark_report_json_path()?,
	)?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.agent_knowledge_os_closeout_benchmark_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1023"));
	assert_eq!(
		report.pointer("/summary/strongest_measured_integrated_product").and_then(Value::as_str),
		Some("ELF integrated Agent Knowledge OS")
	);
	assert_eq!(
		report.pointer("/all_project_fixture_rerun/status").and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report.pointer("/all_project_fixture_rerun/job_count").and_then(Value::as_u64),
		Some(62)
	);
	assert_eq!(report.pointer("/all_project_fixture_rerun/pass").and_then(Value::as_u64), Some(55));
	assert_eq!(report.pointer("/summary/product_count").and_then(Value::as_u64), Some(19));
	assert_eq!(report.pointer("/summary/scenario_count").and_then(Value::as_u64), Some(6));
	assert_eq!(
		report
			.pointer("/summary/not_every_product_has_complete_live_coverage")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report.pointer("/summary/evidence_class_counts/pass").and_then(Value::as_u64),
		Some(9)
	);
	assert_eq!(
		report.pointer("/summary/evidence_class_counts/not_tested").and_then(Value::as_u64),
		Some(78)
	);

	let scenarios = support::array_at(&report, "/supported_scenarios")?;
	let matrix = support::array_at(&report, "/product_matrix")?;

	for scenario in [
		"source_library_ingest_hydration",
		"memory_authority_history_read_profiles",
		"knowledge_workspace_pages",
		"temporal_topic_graph_lite",
		"dreaming_review_queue",
		"recall_debug_panel",
	] {
		support::find_by_field(scenarios, "/id", scenario)?;
	}

	let elf = support::find_by_field(matrix, "/product", "ELF")?;

	for scenario in [
		"source_library_ingest_hydration",
		"memory_authority_history_read_profiles",
		"knowledge_workspace_pages",
		"temporal_topic_graph_lite",
		"dreaming_review_queue",
		"recall_debug_panel",
	] {
		assert_eq!(
			elf.pointer(&format!("/statuses/{scenario}")).and_then(Value::as_str),
			Some("pass")
		);
	}

	let qmd = support::find_by_field(matrix, "/product", "qmd")?;

	assert_eq!(
		qmd.pointer("/statuses/recall_debug_panel").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert!(
		qmd.pointer("/strongest_advantage")
			.and_then(Value::as_str)
			.is_some_and(|value| value.contains("weighted fusion"))
	);

	for product in ["VectifyAI PageIndex", "VectifyAI OpenKB"] {
		let row = support::find_by_field(matrix, "/product", product)?;

		assert_eq!(row.pointer("/coverage").and_then(Value::as_str), Some("reference_only"));
		assert_eq!(
			row.pointer("/statuses/knowledge_workspace_pages").and_then(Value::as_str),
			Some("not_tested")
		);
	}

	assert_eq!(
		report.pointer("/claim_boundaries/no_broad_superiority_claim").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report
			.pointer("/claim_boundaries/reference_only_projects_do_not_count_as_pass")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert!(support::array_contains_str(
		&report,
		"/source_evidence",
		"https://github.com/VectifyAI/PageIndex"
	)?);
	assert!(support::array_contains_str(
		&report,
		"/source_evidence",
		"https://github.com/VectifyAI/OpenKB"
	)?);

	Ok(())
}

#[test]
fn agent_knowledge_os_closeout_benchmark_wires_docs_and_optimization_queue() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::agent_knowledge_os_closeout_benchmark_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::agent_knowledge_os_closeout_benchmark_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let queue = support::array_at(&report, "/optimization_queue")?;

	for item in queue {
		assert_eq!(item.pointer("/generated_from_delta").and_then(Value::as_bool), Some(true));
	}
	for key in [
		"pageindex_openkb_source_library_adapter",
		"qmd_retrieval_knobs_and_short_replay",
		"operator_knowledge_library_ui",
		"openviking_context_trajectory_artifacts",
		"graph_rag_temporal_adapter_matrix",
	] {
		let item = support::find_by_field(queue, "/key", key)?;

		assert_eq!(item.pointer("/generated_from_delta").and_then(Value::as_bool), Some(true));
	}

	assert!(markdown.contains("ELF is the strongest measured integrated product"));
	assert!(markdown.contains("complete live coverage"));
	assert!(markdown.contains("VectifyAI PageIndex"));
	assert!(markdown.contains("VectifyAI OpenKB"));
	assert!(markdown.contains("Do not claim ELF broadly beats every competitor"));
	assert!(
		benchmarking_index.contains("2026-06-20-agent-knowledge-os-closeout-benchmark-report.md")
	);
	assert!(readme.contains("Agent Knowledge OS closeout after XY-1023"));
	assert!(readme.contains("62 jobs, 55 pass"));
	assert!(readme.contains("VectifyAI PageIndex/OpenKB"));
	assert!(readme.contains("strongest measured integrated"));

	Ok(())
}
