use std::fs;

use color_eyre::Result;

use crate::{support, trace_replay_reports::trace_replay_reports_source_scan};

#[test]
fn graph_topic_map_report_wires_source_backed_graph_lite_readback() -> Result<()> {
	let markdown = fs::read_to_string(support::graph_topic_map_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let workspace = support::workspace_root()?;
	let graph_report_service =
		trace_replay_reports_source_scan::graph_report_service_sources(&workspace)?;
	let api_routes = trace_replay_reports_source_scan::api_route_sources(&workspace)?;
	let mcp_server = trace_replay_reports_source_scan::mcp_server_sources(&workspace)?;
	let graph_spec = fs::read_to_string(
		support::workspace_root()?.join("docs/spec/system_graph_memory_postgres_v1.md"),
	)?;

	assert!(markdown.contains("Graph Topic-Map Report - June 20, 2026"));
	assert!(markdown.contains("elf.graph_report/v1"));
	assert!(markdown.contains("sourced"));
	assert!(markdown.contains("inferred"));
	assert!(markdown.contains("ambiguous"));
	assert!(markdown.contains("stale"));
	assert!(markdown.contains("superseded"));
	assert!(markdown.contains("valid_from"));
	assert!(markdown.contains("valid_to"));
	assert!(markdown.contains("valid_at"));
	assert!(markdown.contains("invalid_at"));
	assert!(graph_report_service.contains("ELF_GRAPH_REPORT_SCHEMA_V1"));
	assert!(graph_report_service.contains("GraphReportSummary"));
	assert!(graph_report_service.contains("build_topic_map"));
	assert!(api_routes.contains("/v2/graph/report"));
	assert!(mcp_server.contains("elf_graph_report"));
	assert!(graph_spec.contains("elf.graph_report/v1"));
	assert!(graph_spec.contains("Graphiti/Zep `valid_at` and `invalid_at`"));
	assert!(benchmarking_index.contains("2026-06-20-graph-topic-map-report.md"));
	assert!(readme.contains("Graph Topic-Map Report - June 20, 2026"));
	assert!(readme.contains("Graph topic-map reports after XY-1020"));

	Ok(())
}
