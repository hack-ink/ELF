use std::{fs, path::Path};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

fn graph_report_service_sources(workspace: &Path) -> Result<String> {
	let mut source =
		fs::read_to_string(workspace.join("packages/elf-service/src/graph_report.rs"))?;

	append_rust_sources(
		workspace.join("packages/elf-service/src/graph_report").as_path(),
		&mut source,
	)?;

	Ok(source)
}

fn mcp_server_sources(workspace: &Path) -> Result<String> {
	let mut source = fs::read_to_string(workspace.join("apps/elf-mcp/src/app/server.rs"))?;

	append_rust_sources(workspace.join("apps/elf-mcp/src/app/server").as_path(), &mut source)?;

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
fn graph_topic_map_report_wires_source_backed_graph_lite_readback() -> Result<()> {
	let markdown = fs::read_to_string(support::graph_topic_map_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let workspace = support::workspace_root()?;
	let graph_report_service = graph_report_service_sources(&workspace)?;
	let api_routes =
		fs::read_to_string(support::workspace_root()?.join("apps/elf-api/src/routes.rs"))?;
	let mcp_server = mcp_server_sources(&workspace)?;
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

#[test]
fn qmd_trace_replay_diagnostics_report_preserves_claim_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::trace_replay_diagnostics_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::trace_replay_diagnostics_markdown_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let adoption_report = fs::read_to_string(support::competitor_strength_adoption_report_path()?)?;
	let adoption_json = serde_json::from_str::<Value>(&fs::read_to_string(
		support::competitor_strength_adoption_report_json_path()?,
	)?)?;

	assert_trace_replay_diagnostics_json(&report)?;
	assert_trace_replay_diagnostics_markdown(&markdown);

	assert!(readme.contains("ELF/qmd Trace Replay Diagnostics Report - June 11, 2026"));
	assert!(benchmarking_index.contains("2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"));
	assert!(benchmarking_index.contains("qmd top-10/replay artifact"));
	assert!(benchmarking_index.contains("ELF trace/admin surfaces"));
	assert!(adoption_report.contains("| Retrieval quality and local debug UX | `loss` |"));
	assert!(adoption_report.contains("Letta scenario rows remain"));
	assert!(adoption_report.contains("blocked or `not_tested`"));

	assert_trace_replay_viewer_blocker_boundaries(
		&readme,
		&markdown,
		&adoption_report,
		&report,
		&adoption_json,
	)?;

	assert!(
		adoption_report
			.contains("Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF")
	);
	assert!(support::array_at(&adoption_json, "/adoption_decision/remaining_caveats")?.iter().any(
		|caveat| {
			caveat.as_str().is_some_and(|text| {
				text.contains("Letta scenario rows remain blocked or not_tested")
			})
		}
	));

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
		support::string_array_at(report, "/outcome_terms")?,
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
	assert_eq!(report.pointer("/summary/outcome_counts/win").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/outcome_counts/tie").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/outcome_counts/non_goal").and_then(Value::as_u64), Some(1));

	assert_trace_replay_diagnostics_scenarios(report)
}

fn assert_trace_replay_diagnostics_scenarios(report: &Value) -> Result<()> {
	let scenarios = support::array_at(report, "/scenario_outcomes")?;
	let retrieval =
		support::find_by_field(scenarios, "/scenario_id", "retrieval_correctness_guardrail")?;
	let top10 =
		support::find_by_field(scenarios, "/scenario_id", "default_top10_candidate_artifact")?;
	let replay = support::find_by_field(scenarios, "/scenario_id", "replay_command_locality")?;
	let trace_surface = support::find_by_field(
		scenarios,
		"/scenario_id",
		"trace_admin_replay_surface_availability",
	)?;
	let operator_trace =
		support::find_by_field(scenarios, "/scenario_id", "operator_debug_trace_hydration")?;
	let operator_replay = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_replay_command_availability",
	)?;
	let operator_candidate = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let operator_repair =
		support::find_by_field(scenarios, "/scenario_id", "operator_debug_repair_action_clarity")?;
	let operator_selected = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_selected_but_not_narrated",
	)?;
	let expansion =
		support::find_by_field(scenarios, "/scenario_id", "query_expansion_attribution")?;
	let dense_sparse =
		support::find_by_field(scenarios, "/scenario_id", "dense_sparse_channel_attribution")?;
	let fusion = support::find_by_field(scenarios, "/scenario_id", "fusion_attribution")?;
	let rerank = support::find_by_field(scenarios, "/scenario_id", "rerank_attribution")?;
	let candidate_drop =
		support::find_by_field(scenarios, "/scenario_id", "candidate_drop_diagnostics")?;
	let selected = support::find_by_field(
		scenarios,
		"/scenario_id",
		"selected_but_not_narrated_wrong_results",
	)?;
	let tombstone =
		support::find_by_field(scenarios, "/scenario_id", "evidence_absent_tombstone_diagnostics")?;

	assert_eq!(scenarios.len(), 16);
	assert_eq!(retrieval.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(top10.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(replay.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(trace_surface.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(
		operator_trace.pointer("/evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(operator_trace.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(operator_trace.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(operator_replay.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_candidate.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(support::array_contains_str(
		operator_candidate,
		"/typed_non_pass_states",
		"retrieved_but_dropped"
	)?);
	assert_eq!(operator_repair.pointer("/outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_selected.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(support::array_contains_str(
		operator_selected,
		"/typed_non_pass_states",
		"selected_but_not_narrated"
	)?);
	assert_eq!(expansion.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(dense_sparse.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(fusion.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(rerank.pointer("/result_type").and_then(Value::as_str), Some("non_goal"));
	assert_eq!(rerank.pointer("/outcome").and_then(Value::as_str), Some("non_goal"));
	assert_eq!(candidate_drop.pointer("/outcome").and_then(Value::as_str), Some("not_tested"));
	assert!(support::array_contains_str(
		candidate_drop,
		"/typed_non_pass_states",
		"retrieved_but_dropped"
	)?);
	assert_eq!(selected.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert!(support::array_contains_str(
		selected,
		"/typed_non_pass_states",
		"selected_but_not_narrated"
	)?);
	assert_eq!(tombstone.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert_eq!(tombstone.pointer("/qmd_status").and_then(Value::as_str), Some("wrong_result"));
	assert!(support::array_contains_str(
		report,
		"/wrong_result_diagnostics/qmd_missing_evidence",
		"delete-tombstone"
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"qmd currently wins the default local-debug artifact surface: top-10 rows plus short CLI replay."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"ELF narrowly wins the live operator-debug trace hydration and candidate-drop visibility slice against qmd; qmd still ties replay-command and repair-action clarity."
	)?);
	assert!(support::array_contains_str(
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
	assert!(
		markdown
			.contains("| Operator-debug trace hydration | `live_real_world` | `pass` | `win` |")
	);
	assert!(markdown.contains(
		"| Operator-debug replay command availability | `live_real_world` | `pass` | `tie` |"
	));
	assert!(markdown.contains(
		"| Operator-debug candidate-drop visibility | `live_real_world` | `pass` | `win` |"
	));
	assert!(markdown.contains("| Rerank attribution | `live_baseline_only` | `non_goal` |"));
	assert!(markdown.contains("| Candidate-drop diagnostics | `research_gate` | `not_encoded` |"));
	assert!(markdown.contains("`retrieved_but_dropped` | Defined globally as `not_tested`"));
	assert!(markdown.contains("npx tsx src/cli/qmd.ts query"));
	assert!(markdown.contains("cargo run -p elf-eval -- --config-a"));
	assert!(markdown.contains("cargo make real-world-job-operator-ux-live-adapters"));
	assert!(markdown.contains("Do not claim qmd beats ELF as a memory system overall"));
	assert!(markdown.contains("Do not score rerank superiority from a qmd `--no-rerank` run"));
}

fn assert_trace_replay_viewer_blocker_boundaries(
	readme: &str,
	markdown: &str,
	adoption_report: &str,
	report: &Value,
	adoption_json: &Value,
) -> Result<()> {
	let checked_surfaces = [
		support::collapse_whitespace(readme),
		support::collapse_whitespace(markdown),
		support::collapse_whitespace(adoption_report),
		report.to_string(),
		adoption_json.to_string(),
	];

	for surface in checked_surfaces {
		assert!(!surface.contains("blocked or not encoded"));
	}

	assert!(
		support::collapse_whitespace(readme)
			.contains("claude-mem viewer flows remain blocked until Docker-contained")
	);
	assert!(
		support::collapse_whitespace(markdown)
			.contains("claude-mem UI repair paths remain blocked until Docker-contained")
	);
	assert!(
		support::collapse_whitespace(adoption_report)
			.contains("claude-mem viewer workflows remain blocked until Docker-contained")
	);

	Ok(())
}

fn assert_trace_replay_adoption_json(adoption: &Value) -> Result<()> {
	let local_debug = support::find_by_field(
		support::array_at(adoption, "/scenario_outcomes")?,
		"/scenario_id",
		"local_debug_replay_ux",
	)?;
	let operator_debug = support::find_by_field(
		support::array_at(adoption, "/scenario_outcomes")?,
		"/scenario_id",
		"operator_debugging_viewer_ux",
	)?;

	assert_eq!(local_debug.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert!(
		local_debug
			.pointer("/measured_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd stronger on immediate top-10"))
	);
	assert!(support::array_contains_str(
		local_debug,
		"/command_artifacts",
		"docs/evidence/benchmarking/2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"
	)?);
	assert!(support::array_contains_str(
		adoption,
		"/claim_boundaries/not_allowed",
		"Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF memory-system or retrieval-quality win."
	)?);
	assert_eq!(operator_debug.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(
		operator_debug
			.pointer("/measured_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("narrow live operator-debug win over qmd"))
	);
	assert!(support::array_contains_str(
		operator_debug,
		"/command_artifacts",
		"tmp/real-world-job/operator-ux-live-adapters/summary.json"
	)?);
	assert!(support::array_contains_str(
		adoption,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF broadly beats OpenMemory or claude-mem viewer UX from the narrow ELF/qmd operator-debug slice."
	)?);

	Ok(())
}
