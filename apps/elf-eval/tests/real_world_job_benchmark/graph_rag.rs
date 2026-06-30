use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_graphify_adapter(adapter: &Value) -> Result<()> {
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

pub(super) fn assert_graph_rag_representative_scenarios(
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
		.arg(support::fixture_dir())
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
	let adapters = support::array_at(&report, "/external_adapters/adapters")?;
	let graphify = support::find_by_field(adapters, "/adapter_id", "graphify_docker_smoke")?;
	let suites = support::array_at(graphify, "/suites")?;
	let retrieval = support::find_by_field(suites, "/suite_id", "retrieval")?;

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
	let report = support::run_json_report_from(support::graph_rag_external_fixture_dir())?;

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

	let jobs = support::array_at(&report, "/jobs")?;
	let ragflow =
		support::find_by_field(jobs, "/job_id", "graph-rag-ragflow-reference-chunks-001")?;
	let lightrag =
		support::find_by_field(jobs, "/job_id", "graph-rag-lightrag-context-sources-001")?;
	let graphrag = support::find_by_field(jobs, "/job_id", "graph-rag-graphrag-output-tables-001")?;
	let graphiti =
		support::find_by_field(jobs, "/job_id", "graph-rag-graphiti-temporal-validity-001")?;
	let graphify = support::find_by_field(jobs, "/job_id", "graph-rag-graphify-graph-report-001")?;

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
	assert!(support::array_contains_str(
		graphiti,
		"/produced_evidence",
		"graphiti-current-fact-contract"
	)?);
	assert!(support::array_contains_str(
		graphiti,
		"/produced_evidence",
		"graphiti-historical-fact-contract"
	)?);
	assert!(support::array_contains_str(
		graphiti,
		"/produced_evidence",
		"graphiti-provider-boundary"
	)?);
	assert!(support::array_contains_str(
		graphify,
		"/produced_evidence",
		"graphify-source-location-output"
	)?);

	Ok(())
}
