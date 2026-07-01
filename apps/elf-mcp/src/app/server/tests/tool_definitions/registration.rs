use crate::app::server::tests::tool_definitions::catalog;

#[test]
fn registers_all_tools() {
	let tools = catalog::build_tools();
	let expected = [
		"elf_notes_ingest",
		"elf_graph_query",
		"elf_graph_report",
		"elf_events_ingest",
		"elf_core_blocks_get",
		"elf_entity_memory_get",
		"elf_searches_create",
		"elf_searches_get",
		"elf_searches_timeline",
		"elf_searches_notes",
		"elf_notes_list",
		"elf_notes_get",
		"elf_notes_patch",
		"elf_notes_delete",
		"elf_notes_publish",
		"elf_notes_unpublish",
		"elf_space_grants_list",
		"elf_space_grant_upsert",
		"elf_space_grant_revoke",
		"elf_admin_traces_recent_list",
		"elf_dreaming_review_queue",
		"elf_recall_debug_panel",
		"elf_work_journal_entry_create",
		"elf_work_journal_entry_get",
		"elf_work_journal_session_readback",
		"elf_admin_trace_get",
		"elf_admin_trajectory_get",
		"elf_admin_trace_item_get",
		"elf_admin_note_provenance_get",
		"elf_admin_memory_history_get",
		"elf_admin_trace_bundle_get",
		"elf_admin_events_ingestion_profiles_list",
		"elf_admin_events_ingestion_profiles_create",
		"elf_admin_events_ingestion_profile_get",
		"elf_admin_events_ingestion_profile_versions_list",
		"elf_admin_events_ingestion_profile_default_get",
		"elf_admin_events_ingestion_profile_default_set",
	];

	for name in expected {
		assert!(tools.contains_key(name), "Missing tool registration: {name}.");
	}

	assert_eq!(tools.len(), expected.len(), "Unexpected tool count for MCP registration.");
}
