use super::*;

#[test]
fn agent_knowledge_os_closeout_benchmark_preserves_full_matrix_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		agent_knowledge_os_closeout_benchmark_report_json_path()?,
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

	let scenarios = array_at(&report, "/supported_scenarios")?;
	let matrix = array_at(&report, "/product_matrix")?;

	for scenario in [
		"source_library_ingest_hydration",
		"memory_authority_history_read_profiles",
		"knowledge_workspace_pages",
		"temporal_topic_graph_lite",
		"dreaming_review_queue",
		"recall_debug_panel",
	] {
		find_by_field(scenarios, "/id", scenario)?;
	}

	let elf = find_by_field(matrix, "/product", "ELF")?;

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

	let qmd = find_by_field(matrix, "/product", "qmd")?;

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
		let row = find_by_field(matrix, "/product", product)?;

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
	assert!(array_contains_str(
		&report,
		"/source_evidence",
		"https://github.com/VectifyAI/PageIndex"
	)?);
	assert!(array_contains_str(
		&report,
		"/source_evidence",
		"https://github.com/VectifyAI/OpenKB"
	)?);

	Ok(())
}

#[test]
fn agent_knowledge_os_closeout_benchmark_wires_docs_and_optimization_queue() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		agent_knowledge_os_closeout_benchmark_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(agent_knowledge_os_closeout_benchmark_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;
	let queue = array_at(&report, "/optimization_queue")?;

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
		let item = find_by_field(queue, "/key", key)?;

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

#[test]
fn p2_knowledge_workspace_closeout_preserves_pageindex_openkb_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		p2_knowledge_workspace_pageindex_openkb_closeout_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(
		p2_knowledge_workspace_pageindex_openkb_closeout_report_markdown_path()?,
	)?;
	let makefile = fs::read_to_string(workspace_root()?.join("Makefile.toml"))?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;
	let benchmark_runbook = fs::read_to_string(
		workspace_root()?
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("real_world_agent_memory_benchmark.md"),
	)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.p2_knowledge_workspace_pageindex_openkb_closeout_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1066"));
	assert_eq!(
		report.pointer("/self_assessment/verdict").and_then(Value::as_str),
		Some("pass_with_reference_only_competitor_boundary")
	);
	assert_eq!(report.pointer("/typed_state_summary/pass").and_then(Value::as_u64), Some(2));
	assert_eq!(
		report.pointer("/typed_state_summary/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/typed_state_summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/typed_state_summary/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/typed_state_summary/not_tested").and_then(Value::as_u64), Some(2));

	let results = array_at(&report, "/elf_same_corpus_results")?;
	let source_library = find_by_field(results, "/suite", "source_library")?;
	let knowledge = find_by_field(results, "/suite", "knowledge_compilation")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/jobs").and_then(Value::as_u64), Some(2));
	assert_eq!(knowledge.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(knowledge.pointer("/jobs").and_then(Value::as_u64), Some(3));
	assert!(array_contains_str(
		knowledge,
		"/coverage",
		"Changed-source watch/rebuild reports changed, stale, and reviewable memory-candidate outputs without source mutation."
	)?);

	let matrix = array_at(&report, "/comparison_matrix")?;
	let pageindex = find_by_field(matrix, "/target", "VectifyAI PageIndex")?;
	let openkb = find_by_field(matrix, "/target", "VectifyAI OpenKB")?;
	let p3 = find_by_field(matrix, "/target", "P3 PageIndex/OpenKB adapter queue")?;

	assert_eq!(pageindex.pointer("/status").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(openkb.pointer("/status").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(p3.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		report
			.pointer("/p3_queue_decision/ready_to_queue_after_main_thread_acceptance")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report.pointer("/p3_queue_decision/queued_label_applied").and_then(Value::as_bool),
		Some(false)
	);
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF beats PageIndex or OpenKB."
	)?);
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not queue a P3 issue in this lane."
	)?);
	assert!(markdown.contains("P2 Knowledge Workspace PageIndex/OpenKB Closeout Report"));
	assert!(markdown.contains("VectifyAI PageIndex"));
	assert!(markdown.contains("VectifyAI OpenKB"));
	assert!(markdown.contains("This report does not apply `decodex:queued:elf`"));
	assert!(makefile.contains("[tasks.real-world-memory-p2-knowledge-closeout]"));
	assert!(makefile.contains("\"real-world-memory-source-library-report\""));
	assert!(makefile.contains("\"real-world-memory-knowledge-report\""));
	assert!(
		benchmarking_index
			.contains("2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md")
	);
	assert!(readme.contains("P2 Knowledge Workspace PageIndex/OpenKB closeout after XY-1066"));
	assert!(readme.contains("real-world-memory-p2-knowledge-closeout"));
	assert!(benchmark_runbook.contains("cargo make real-world-memory-p2-knowledge-closeout"));

	Ok(())
}

#[test]
fn operator_approved_public_proxy_private_addendum_preserves_boundary() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		operator_approved_public_proxy_private_addendum_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(
		operator_approved_public_proxy_private_addendum_report_markdown_path()?,
	)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.operator_approved_public_proxy_baseline_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-930"));
	assert_eq!(report.pointer("/command/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		report.pointer("/command/run_id").and_then(Value::as_str),
		Some("live-baseline-20260619143959")
	);
	assert_eq!(
		report.pointer("/corpus/profile").and_then(Value::as_str),
		Some("production-private")
	);
	assert_eq!(
		report.pointer("/corpus/runner_track").and_then(Value::as_str),
		Some("private_production")
	);
	assert_eq!(
		report.pointer("/corpus/manifest_kind").and_then(Value::as_str),
		Some("operator_approved_public_proxy")
	);
	assert_eq!(
		report.pointer("/corpus/manifest_id").and_then(Value::as_str),
		Some("operator-approved-public-proxy-prod-corpus-2026-06-19")
	);
	assert_eq!(report.pointer("/embedding/mode").and_then(Value::as_str), Some("local"));
	assert_eq!(
		report.pointer("/embedding/provider_backed_quality_proven").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(report.pointer("/summary/project_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/check_summary/total").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/check_summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(
		report.pointer("/query_summary/wrong_result_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/backfill/completed_count").and_then(Value::as_u64), Some(12));
	assert_eq!(report.pointer("/backfill/duplicate_source_notes").and_then(Value::as_u64), Some(0));

	let queries = array_at(&report, "/queries")?;
	let provider = find_by_field(queries, "/id", "q-explain-provider-blocker")?;

	assert_eq!(queries.len(), 8);
	assert_eq!(
		provider.pointer("/top_evidence").and_then(Value::as_str),
		Some("blocker-provider-missing")
	);
	assert_eq!(provider.pointer("/matched").and_then(Value::as_bool), Some(true));
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not call this real private-corpus production proof."
	)?);
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim provider-backed production quality; embedding mode was local."
	)?);
	assert!(array_contains_str(
		&report,
		"/improvement_regression_readback/unchanged",
		"Real private-corpus production quality is still not proven."
	)?);
	assert!(array_contains_str(
		&report,
		"/next_optimization_direction/when_operator_inputs_exist",
		"Run provider-backed embeddings with ELF_BASELINE_ELF_EMBEDDING_MODE=provider and a routed provider setup."
	)?);
	assert!(markdown.contains("proxy corpus pass"));
	assert!(markdown.contains("Do not call this real private-corpus production proof."));
	assert!(markdown.contains("| Embedding mode | `local` |"));
	assert!(
		benchmarking_index
			.contains("2026-06-19-operator-approved-public-proxy-production-private-addendum.md")
	);
	assert!(benchmarking_index.contains("not real private-corpus or provider-backed proof"));
	assert!(readme.contains("Operator-approved public-proxy addendum after XY-930"));
	assert!(readme.contains("8/8 query passes"));
	assert!(readme.contains("does not prove real private-corpus production quality"));

	Ok(())
}

#[test]
fn openmemory_ui_export_product_recheck_preserves_blocked_boundary() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		openmemory_ui_export_product_readback_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(openmemory_ui_export_product_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.openmemory_ui_export_product_recheck_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-987"));
	assert_eq!(
		report.pointer("/command/command").and_then(Value::as_str),
		Some("cargo make openmemory-ui-export-readback")
	);
	assert_eq!(report.pointer("/command/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		report.pointer("/command/probe_artifact").and_then(Value::as_str),
		Some("tmp/live-baseline/mem0-openmemory-ui-export.json")
	);
	assert_eq!(report.pointer("/run/sdk_check_summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/run/ui_export_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		report.pointer("/run/ui_export_reason_code").and_then(Value::as_str),
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
			.pointer("/openmemory_product_surface/export_requires_running_container")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert!(
		report
			.pointer("/openmemory_probe/attempt/output_excerpt")
			.and_then(Value::as_str)
			.is_some_and(|excerpt| excerpt.contains("docker: command not found")
				&& excerpt.contains("Container 'openmemory-openmemory-mcp-1' not found/running"))
	);
	assert_eq!(
		report.pointer("/classification/comparison_judgment").and_then(Value::as_str),
		Some("unchanged")
	);
	assert_eq!(
		report
			.pointer("/claim_boundary/product_browser_or_dashboard_readback_reached")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert!(array_contains_str(
		&report,
		"/improvement_regression_readback/unchanged",
		"OpenMemory product UI/export readback remains blocked before same-corpus product app database validation."
	)?);
	assert!(array_contains_str(
		&report,
		"/next_optimization_direction/required_fields",
		"same_corpus_import_into_openmemory_app_database"
	)?);
	assert!(markdown.contains("OpenMemory UI/export product-readback status is unchanged"));
	assert!(markdown.contains("Product browser/dashboard readback reached"));
	assert!(
		benchmarking_index.contains("2026-06-19-openmemory-ui-export-product-readback-report.md")
	);
	assert!(readme.contains("OpenMemory UI/Export Product Readback Report - June 19, 2026"));
	assert!(readme.contains("OpenMemory UI/export product recheck after XY-987"));

	Ok(())
}

#[test]
fn graph_rag_citation_navigation_promotion_preserves_typed_non_passes() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		graph_rag_citation_navigation_promotion_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(graph_rag_citation_navigation_promotion_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

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

	let scenarios = array_at(&report, "/scenario_outcomes")?;
	let ragflow = find_by_field(scenarios, "/project", "RAGFlow")?;
	let lightrag = find_by_field(scenarios, "/project", "LightRAG")?;
	let graphrag = find_by_field(scenarios, "/project", "GraphRAG")?;
	let graphiti = find_by_field(scenarios, "/project", "Graphiti/Zep")?;
	let graphify = find_by_field(scenarios, "/project", "graphify")?;
	let llm_wiki = find_by_field(scenarios, "/project", "llm-wiki")?;
	let gbrain = find_by_field(scenarios, "/project", "gbrain")?;

	assert_eq!(ragflow.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(lightrag.pointer("/current_status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(graphrag.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphiti.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphify.pointer("/current_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(llm_wiki.pointer("/current_status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(gbrain.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert!(array_contains_str(graphify, "/produced_evidence", "graphify-source-location-output")?);
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim graph/RAG parity or broad graph-navigation quality."
	)?);
	assert!(array_contains_str(
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
		graph_rag_adapter_matrix_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(graph_rag_adapter_matrix_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

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

	let rows = array_at(&report, "/adapter_matrix")?;
	let ragflow_citation = find_matrix_row(rows, "RAGFlow", "citation_quality")?;
	let lightrag_retrieval = find_matrix_row(rows, "LightRAG", "retrieval_quality")?;
	let graphrag_navigation = find_matrix_row(rows, "GraphRAG", "navigation_quality")?;
	let graphrag_retrieval = find_matrix_row(rows, "GraphRAG", "retrieval_quality")?;

	assert_eq!(ragflow_citation.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(lightrag_retrieval.pointer("/status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(graphrag_navigation.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphrag_retrieval.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert!(array_contains_str(
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

#[test]
fn p3_competitor_strength_absorption_report_preserves_claim_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		p3_competitor_strength_absorption_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(p3_competitor_strength_absorption_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.p3_competitor_strength_absorption_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1072"));
	assert_eq!(
		report.pointer("/self_assessment/verdict").and_then(Value::as_str),
		Some("pass_with_p4_queue_ready_after_main_thread_acceptance")
	);
	assert_eq!(
		report.pointer("/self_assessment/p4_queued_label_applied").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report
			.pointer("/self_assessment/typed_non_pass_states_are_not_wins")
			.and_then(Value::as_bool),
		Some(true)
	);

	let products = array_at(&report, "/product_strengths")?;

	for product in [
		"qmd",
		"VectifyAI PageIndex",
		"VectifyAI OpenKB",
		"mem0/OpenMemory",
		"Letta",
		"Graphiti/Zep",
		"OpenViking",
		"RAGFlow",
		"GraphRAG",
		"LightRAG",
	] {
		find_by_field(products, "/product", product)?;
	}

	let qmd = find_by_field(products, "/product", "qmd")?;
	let pageindex = find_by_field(products, "/product", "VectifyAI PageIndex")?;
	let mem0 = find_by_field(products, "/product", "mem0/OpenMemory")?;
	let graphiti = find_by_field(products, "/product", "Graphiti/Zep")?;
	let lightrag = find_by_field(products, "/product", "LightRAG")?;

	assert_eq!(qmd.pointer("/current_status").and_then(Value::as_str), Some("mixed"));
	assert!(
		qmd.pointer("/remains_stronger_elsewhere")
			.and_then(Value::as_str)
			.is_some_and(|value| value.contains("top-k JSON"))
	);
	assert_eq!(pageindex.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		mem0.pointer("/current_status").and_then(Value::as_str),
		Some("split_pass_and_blocked")
	);
	assert_eq!(graphiti.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		lightrag.pointer("/current_status").and_then(Value::as_str),
		Some("incomplete_or_not_encoded")
	);

	let queue = array_at(&report, "/p4_optimization_queue")?;

	for key in [
		"qmd_candidate_replay_parity",
		"adapter_outcome_grammar_and_metrics",
		"source_library_tree_and_wiki_adapters",
		"memory_history_export_and_core_archive",
		"temporal_trajectory_graph_rag_adapters",
	] {
		let item = find_by_field(queue, "/key", key)?;

		assert_eq!(
			item.pointer("/ready_after_main_thread_acceptance").and_then(Value::as_bool),
			Some(true)
		);
		assert_eq!(item.pointer("/queued_label_applied").and_then(Value::as_bool), Some(false));
	}

	assert_product_queue_items_reference_queue(products, queue)?;

	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Typed non-pass states are not wins."
	)?);
	assert!(array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not apply decodex:queued:elf to a P4 issue until the main thread accepts the P3 closeout."
	)?);
	assert!(markdown.contains("P3 is decision-ready for main-thread inspection"));
	assert!(markdown.contains("Typed non-pass states are not wins"));
	assert!(markdown.contains("No P4 issue receives `decodex:queued:elf`"));
	assert!(benchmarking_index.contains("2026-06-23-p3-competitor-strength-absorption-report.md"));
	assert!(readme.contains("P3 competitor-strength absorption closeout after XY-1072"));
	assert!(readme.contains("`decodex:queued:elf` label"));

	Ok(())
}

fn assert_product_queue_items_reference_queue(products: &[Value], queue: &[Value]) -> Result<()> {
	let queue_keys = queue
		.iter()
		.filter_map(|item| item.pointer("/key").and_then(Value::as_str))
		.collect::<Vec<_>>();

	for product in products {
		let product_name = product
			.pointer("/product")
			.and_then(Value::as_str)
			.ok_or_else(|| eyre::eyre!("product row is missing product name"))?;
		let queue_item = product
			.pointer("/p4_queue_item")
			.and_then(Value::as_str)
			.ok_or_else(|| eyre::eyre!("product {product_name} is missing p4_queue_item"))?;

		assert!(
			queue_keys.contains(&queue_item),
			"product {product_name} references missing P4 queue item {queue_item}"
		);
	}

	Ok(())
}

fn find_matrix_row<'a>(rows: &'a [Value], adapter: &str, dimension: &str) -> Result<&'a Value> {
	rows.iter()
		.find(|row| {
			row.pointer("/adapter").and_then(Value::as_str) == Some(adapter)
				&& row.pointer("/dimension").and_then(Value::as_str) == Some(dimension)
		})
		.ok_or_else(|| eyre::eyre!("missing matrix row for {adapter} {dimension}"))
}
