use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(in super::super) fn assert_graphify_adapter(adapter: &Value) -> Result<()> {
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

	let capabilities = support::array_at(adapter, "/capabilities")?;
	let quality = support::find_by_field(capabilities, "/capability", "quality_or_scale_claim")?;

	assert_eq!(quality.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(support::array_at(adapter, "/notes")?.iter().any(|note| {
		note.as_str().is_some_and(|text| text.contains("tiny smoke") && text.contains("non-pass"))
	}));

	Ok(())
}

pub(in super::super) fn assert_graph_rag_representative_scenarios(
	ragflow: &Value,
	lightrag: &Value,
	graphrag: &Value,
	graphiti_zep: &Value,
	graphify: &Value,
) -> Result<()> {
	let ragflow_scenarios = support::array_at(ragflow, "/scenarios")?;
	let lightrag_scenarios = support::array_at(lightrag, "/scenarios")?;
	let graphrag_scenarios = support::array_at(graphrag, "/scenarios")?;
	let graphiti_scenarios = support::array_at(graphiti_zep, "/scenarios")?;
	let graphify_scenarios = support::array_at(graphify, "/scenarios")?;
	let ragflow_chunk = support::find_by_field(
		ragflow_scenarios,
		"/scenario_id",
		"reference_chunk_citation_mapping",
	)?;
	let lightrag_context = support::find_by_field(
		lightrag_scenarios,
		"/scenario_id",
		"context_source_reference_mapping",
	)?;
	let graphrag_tables = support::find_by_field(
		graphrag_scenarios,
		"/scenario_id",
		"output_table_citation_mapping",
	)?;
	let graphiti_temporal = support::find_by_field(
		graphiti_scenarios,
		"/scenario_id",
		"temporal_validity_window_mapping",
	)?;
	let graphify_lint =
		support::find_by_field(graphify_scenarios, "/scenario_id", "graph_report_navigation_lint")?;

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
		let row = support::find_by_field(scenarios, "/scenario_id", scenario_id)?;

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
