use super::*;

fn rust_module_sources(workspace: &Path, root_file: &str, module_dir: &str) -> Result<String> {
	let mut source = fs::read_to_string(workspace.join(root_file))?;

	append_rust_sources(workspace.join(module_dir).as_path(), &mut source)?;

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

fn assert_recall_debug_source_contract(sources: &RecallDebugSourceContract<'_>) {
	assert!(sources.service.contains("ELF_RECALL_DEBUG_PANEL_SCHEMA_V1"));
	assert!(sources.service.contains("ELF_RECALL_TRACE_SCHEMA_V1"));
	assert!(sources.service.contains("pub async fn recall_debug_panel"));
	assert!(sources.service.contains("build_recall_trace"));
	assert!(sources.service.contains("not_requested_layer"));
	assert!(sources.service.contains("blocked_layer"));
	assert!(sources.service.contains("public_error_class"));
	assert!(sources.service.contains("candidate_identity"));
	assert!(sources.service.contains("ORG_PROJECT_ID"));
	assert!(sources.service.contains("trace_bundle_get"));
	assert!(sources.service.contains("docs_search_l0"));
	assert!(sources.service.contains("knowledge_pages_search"));
	assert!(sources.service.contains("graph_report"));
	assert!(sources.service.contains("dreaming_review_queue"));
	assert!(sources.service_lib.contains("pub mod recall_debug"));
	assert!(sources.service_lib.contains("RecallDebugPanelResponse"));
	assert!(sources.service_lib.contains("RecallTrace"));
	assert!(sources.routes.contains("/v2/recall-debug/panel"));
	assert!(sources.routes.contains("/v2/admin/recall-debug/panel"));
	assert!(sources.routes.contains("async fn recall_debug_panel"));
	assert!(sources.routes.contains("RecallDebugPanelRequest"));
	assert!(sources.mcp.contains("elf_recall_debug_panel"));
	assert!(sources.mcp.contains("recall_debug_panel_schema"));
	assert!(sources.mcp.contains("/v2/recall-debug/panel"));
	assert!(sources.recall_spec.contains("elf.recall_debug_panel/v1"));
	assert!(sources.recall_spec.contains("elf.recall_trace/v1"));
	assert!(sources.recall_spec.contains("not_requested"));
	assert!(sources.recall_spec.contains("evidence_class = \"blocked\""));
	assert!(sources.recall_spec.contains("effective `top_k` cap of 32"));
	assert!(sources.recall_spec.contains("context_state = \"stale\""));
	assert!(sources.recall_spec.contains("selected`, `dropped`, `available`, or `reviewable`"));
	assert!(sources.service_spec.contains("POST /v2/recall-debug/panel"));
	assert!(sources.service_spec.contains("POST /v2/admin/recall-debug/panel"));
	assert!(sources.service_spec.contains("elf.recall_trace/v1"));
	assert!(sources.service_spec.contains("system_recall_debug_panel_v1.md"));
	assert!(sources.version_registry.contains("elf.recall_debug_panel/v1"));
	assert!(sources.version_registry.contains("elf.recall_trace/v1"));
	assert!(sources.markdown.contains("Recall Debug Panel Report"));
	assert!(sources.markdown.contains("POST /v2/recall-debug/panel"));
	assert!(sources.markdown.contains("`elf.recall_trace/v1`"));
	assert!(sources.markdown.contains("Missing anchors stay visible as `not_requested`"));
	assert!(sources.markdown.contains("retained dropped replay candidates"));
	assert!(sources.markdown.contains("effective cap of 32 rows"));
	assert!(sources.benchmarking_index.contains("2026-06-20-recall-debug-panel-report.md"));
	assert!(sources.readme.contains("Recall/debug panel after XY-1022"));
	assert!(sources.readme.contains("elf.recall_debug_panel/v1"));
	assert!(sources.readme.contains("retained dropped replay candidates"));
}

#[test]
fn recall_debug_panel_report_wires_cross_layer_debug_contract() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		recall_debug_panel_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(recall_debug_panel_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;
	let workspace = workspace_root()?;
	let service = rust_module_sources(
		&workspace,
		"packages/elf-service/src/recall_debug.rs",
		"packages/elf-service/src/recall_debug",
	)?;
	let service_lib = fs::read_to_string(workspace.join("packages/elf-service/src/lib.rs"))?;
	let routes =
		rust_module_sources(&workspace, "apps/elf-api/src/routes.rs", "apps/elf-api/src/routes")?;
	let mcp =
		rust_module_sources(&workspace, "apps/elf-mcp/src/server.rs", "apps/elf-mcp/src/server")?;
	let recall_spec =
		fs::read_to_string(workspace.join("docs/spec/system_recall_debug_panel_v1.md"))?;
	let service_spec =
		fs::read_to_string(workspace.join("docs/spec/system_elf_memory_service_v2.md"))?;
	let version_registry =
		fs::read_to_string(workspace.join("docs/spec/system_version_registry.md"))?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.recall_debug_panel_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1022"));
	assert_eq!(
		report.pointer("/service_contract/response_schema").and_then(Value::as_str),
		Some("elf.recall_debug_panel/v1")
	);
	assert_eq!(
		report.pointer("/service_contract/trace_schema").and_then(Value::as_str),
		Some("elf.recall_trace/v1")
	);
	assert_eq!(
		report.pointer("/service_contract/read_model_only").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report.pointer("/service_contract/raw_sql_needed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(report.pointer("/layer_contract/layer_count").and_then(Value::as_u64), Some(5));

	let layers = array_at(&report, "/layer_contract/layers")?;

	for (layer, authority, replay) in [
		("memory_notes", "memory_note", "elf_admin_trace_bundle_get"),
		("source_documents", "source_library", "elf_docs_search_l0"),
		("knowledge_pages", "derived_knowledge_page", "elf_recall_debug_panel"),
		("graph_facts", "graph_fact", "elf_graph_report"),
		("dreaming_proposals", "reviewable_dreaming_proposal", "elf_dreaming_review_queue"),
	] {
		let row = find_by_field(layers, "/layer", layer)?;

		assert_eq!(row.pointer("/authority_layer").and_then(Value::as_str), Some(authority));
		assert_eq!(row.pointer("/replay_surface").and_then(Value::as_str), Some(replay));
		assert_eq!(row.pointer("/evidence_class").and_then(Value::as_str), Some("pass"));
	}

	let memory = find_by_field(layers, "/layer", "memory_notes")?;
	let docs = find_by_field(layers, "/layer", "source_documents")?;

	assert!(array_contains_str(memory, "/selection_states", "selected")?);
	assert!(array_contains_str(memory, "/selection_states", "dropped")?);
	assert_eq!(docs.pointer("/effective_limit").and_then(Value::as_u64), Some(32));
	assert_eq!(
		report.pointer("/debug_invariants/not_requested_layers_preserved").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report
			.pointer("/debug_invariants/selected_and_dropped_memory_candidates")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report
			.pointer("/debug_invariants/requested_layer_failures_preserved_as_blocked")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report.pointer("/debug_invariants/no_source_mutation").and_then(Value::as_bool),
		Some(true)
	);

	assert_recall_debug_source_contract(&RecallDebugSourceContract {
		service: &service,
		service_lib: &service_lib,
		routes: &routes,
		mcp: &mcp,
		recall_spec: &recall_spec,
		service_spec: &service_spec,
		version_registry: &version_registry,
		markdown: &markdown,
		benchmarking_index: &benchmarking_index,
		readme: &readme,
	});

	Ok(())
}
