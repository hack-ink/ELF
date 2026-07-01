use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::{closeout_reports::closeout_reports_helpers, support};

#[test]
fn graph_rag_citation_navigation_promotion_preserves_typed_non_passes() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::graph_rag_citation_navigation_promotion_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(
		support::graph_rag_citation_navigation_promotion_report_markdown_path()?,
	)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

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

	let scenarios = support::array_at(&report, "/scenario_outcomes")?;
	let ragflow = support::find_by_field(scenarios, "/project", "RAGFlow")?;
	let lightrag = support::find_by_field(scenarios, "/project", "LightRAG")?;
	let graphrag = support::find_by_field(scenarios, "/project", "GraphRAG")?;
	let graphiti = support::find_by_field(scenarios, "/project", "Graphiti/Zep")?;
	let graphify = support::find_by_field(scenarios, "/project", "graphify")?;
	let llm_wiki = support::find_by_field(scenarios, "/project", "llm-wiki")?;
	let gbrain = support::find_by_field(scenarios, "/project", "gbrain")?;

	assert_eq!(ragflow.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(lightrag.pointer("/current_status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(graphrag.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphiti.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphify.pointer("/current_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(llm_wiki.pointer("/current_status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(gbrain.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert!(support::array_contains_str(
		graphify,
		"/produced_evidence",
		"graphify-source-location-output"
	)?);
	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim graph/RAG parity or broad graph-navigation quality."
	)?);
	assert!(support::array_contains_str(
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

#[test]
fn graph_rag_adapter_matrix_report_preserves_no_parity_claims() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::graph_rag_adapter_matrix_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::graph_rag_adapter_matrix_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.graph_rag_adapter_matrix_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1071"));
	assert_eq!(report.pointer("/summary/matrix_row_count").and_then(Value::as_u64), Some(18));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(6));
	assert_eq!(
		report.pointer("/summary/broad_graph_rag_parity").and_then(Value::as_str),
		Some("not_proven")
	);

	let rows = support::array_at(&report, "/adapter_matrix")?;
	let ragflow_citation =
		closeout_reports_helpers::find_matrix_row(rows, "RAGFlow", "citation_quality")?;
	let lightrag_retrieval =
		closeout_reports_helpers::find_matrix_row(rows, "LightRAG", "retrieval_quality")?;
	let graphrag_navigation =
		closeout_reports_helpers::find_matrix_row(rows, "GraphRAG", "navigation_quality")?;
	let graphrag_retrieval =
		closeout_reports_helpers::find_matrix_row(rows, "GraphRAG", "retrieval_quality")?;

	assert_eq!(ragflow_citation.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(lightrag_retrieval.pointer("/status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(graphrag_navigation.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphrag_retrieval.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not reposition ELF as a generic RAG platform from this adapter matrix."
	)?);
	assert!(markdown.contains("The graph/RAG comparison remains typed non-pass"));
	assert!(markdown.contains("| RAGFlow | `blocked`: answer text plus selected reference chunks"));
	assert!(benchmarking_index.contains("2026-06-23-graph-rag-adapter-matrix-report.md"));
	assert!(readme.contains("RAGFlow/GraphRAG/LightRAG adapter matrix after XY-1071"));
	assert!(readme.contains("Graph/RAG Adapter Matrix Report - June 23, 2026"));

	Ok(())
}
