use std::collections::HashMap;

use crate::app::server::HttpMethod;

const ALL_TOOL_DEFINITIONS: [ToolDefinition; 37] = [
	ToolDefinition::new(
		"elf_notes_ingest",
		HttpMethod::Post,
		"/v2/notes/ingest",
		"Ingest deterministic notes into ELF. This tool never calls an LLM.",
	),
	ToolDefinition::new(
		"elf_graph_query",
		HttpMethod::Post,
		"/v2/graph/query",
		"Query graph entities and relations by structured criteria.",
	),
	ToolDefinition::new(
		"elf_graph_report",
		HttpMethod::Post,
		"/v2/graph/report",
		"Build a source-backed graph topic map with current, historical, future, inferred, ambiguous, stale, and superseded fact markers.",
	),
	ToolDefinition::new(
		"elf_events_ingest",
		HttpMethod::Post,
		"/v2/events/ingest",
		"Ingest an event by extracting evidence-bound notes using the configured LLM extractor.",
	),
	ToolDefinition::new(
		"elf_searches_create",
		HttpMethod::Post,
		"/v2/searches",
		"Create a search session using quick-find or planned-search mode. Response includes optional trajectory_summary.",
	),
	ToolDefinition::new(
		"elf_core_blocks_get",
		HttpMethod::Get,
		"/v2/core-blocks",
		"Fetch core memory blocks explicitly attached to the configured agent and read profile.",
	),
	ToolDefinition::new(
		"elf_entity_memory_get",
		HttpMethod::Get,
		"/v2/entity-memory",
		"Fetch an entity-scoped memory view across attached core blocks and graph-linked archival notes.",
	),
	ToolDefinition::new(
		"elf_dreaming_review_queue",
		HttpMethod::Get,
		"/v2/admin/dreaming/review-queue",
		"List source-backed Dreaming review queue proposals with variants, affected refs, lint flags, policy gates, and review audit.",
	),
	ToolDefinition::new(
		"elf_recall_debug_panel",
		HttpMethod::Post,
		"/v2/recall-debug/panel",
		"Build an agent-facing cross-layer recall/debug panel and deterministic recall_trace over memory traces, source documents, knowledge pages, graph facts, and Dreaming proposals.",
	),
	ToolDefinition::new(
		"elf_work_journal_entry_create",
		HttpMethod::Post,
		"/v2/work-journal/entries",
		"Capture one source-adjacent Work Journal entry with source refs, redaction, next-step, rejected-option, and promotion-boundary metadata.",
	),
	ToolDefinition::new(
		"elf_work_journal_entry_get",
		HttpMethod::Get,
		"/v2/work-journal/entries/{entry_id}",
		"Fetch one readable Work Journal entry by entry_id.",
	),
	ToolDefinition::new(
		"elf_work_journal_session_readback",
		HttpMethod::Post,
		"/v2/work-journal/readback",
		"Read newest Work Journal entries for a session and return a where_stopped projection with journal evidence.",
	),
	ToolDefinition::new(
		"elf_searches_get",
		HttpMethod::Get,
		"/v2/searches/{search_id}",
		"Fetch a search session index view by search_id, including optional trajectory_summary.",
	),
	ToolDefinition::new(
		"elf_searches_timeline",
		HttpMethod::Get,
		"/v2/searches/{search_id}/timeline",
		"Build a timeline view from a search session.",
	),
	ToolDefinition::new(
		"elf_searches_notes",
		HttpMethod::Post,
		"/v2/searches/{search_id}/notes",
		"Fetch note details for selected note_ids from a search session. l0/l1 strip evidence/source_ref/structured; l2 returns full detail.",
	),
	ToolDefinition::new(
		"elf_notes_list",
		HttpMethod::Get,
		"/v2/notes",
		"List notes in a tenant and project with optional filters.",
	),
	ToolDefinition::new(
		"elf_notes_get",
		HttpMethod::Get,
		"/v2/notes/{note_id}",
		"Fetch a single note by note_id.",
	),
	ToolDefinition::new(
		"elf_notes_patch",
		HttpMethod::Patch,
		"/v2/notes/{note_id}",
		"Patch a note by note_id. Only provided fields are updated.",
	),
	ToolDefinition::new(
		"elf_notes_delete",
		HttpMethod::Delete,
		"/v2/notes/{note_id}",
		"Delete a note by note_id.",
	),
	ToolDefinition::new(
		"elf_notes_publish",
		HttpMethod::Post,
		"/v2/notes/{note_id}/publish",
		"Publish a note from agent_private into a shared space (team_shared or org_shared).",
	),
	ToolDefinition::new(
		"elf_notes_unpublish",
		HttpMethod::Post,
		"/v2/notes/{note_id}/unpublish",
		"Unpublish a shared note back into agent_private scope.",
	),
	ToolDefinition::new(
		"elf_space_grants_list",
		HttpMethod::Get,
		"/v2/spaces/{space}/grants",
		"List sharing grants for a space (team_shared or org_shared).",
	),
	ToolDefinition::new(
		"elf_space_grant_upsert",
		HttpMethod::Post,
		"/v2/spaces/{space}/grants",
		"Upsert a sharing grant for a space (team_shared or org_shared).",
	),
	ToolDefinition::new(
		"elf_space_grant_revoke",
		HttpMethod::Post,
		"/v2/spaces/{space}/grants/revoke",
		"Revoke a sharing grant for a space (team_shared or org_shared).",
	),
	ToolDefinition::new(
		"elf_admin_traces_recent_list",
		HttpMethod::Get,
		"/v2/admin/traces/recent",
		"List recent traces by tenant/project with optional cursor and filters.",
	),
	ToolDefinition::new(
		"elf_admin_trace_get",
		HttpMethod::Get,
		"/v2/admin/traces/{trace_id}",
		"Fetch trace metadata, items, and optional trajectory summary by trace_id.",
	),
	ToolDefinition::new(
		"elf_admin_trajectory_get",
		HttpMethod::Get,
		"/v2/admin/trajectories/{trace_id}",
		"Fetch trace trajectory and stage payload by trace_id.",
	),
	ToolDefinition::new(
		"elf_admin_trace_item_get",
		HttpMethod::Get,
		"/v2/admin/trace-items/{item_id}",
		"Fetch a trace item explain payload by item_id.",
	),
	ToolDefinition::new(
		"elf_admin_note_provenance_get",
		HttpMethod::Get,
		"/v2/admin/notes/{note_id}/provenance",
		"Fetch provenance bundle for a note.",
	),
	ToolDefinition::new(
		"elf_admin_memory_history_get",
		HttpMethod::Get,
		"/v2/admin/notes/{note_id}/history",
		"Fetch chronological memory history for a note.",
	),
	ToolDefinition::new(
		"elf_admin_trace_bundle_get",
		HttpMethod::Get,
		"/v2/admin/traces/{trace_id}/bundle",
		"Fetch trace bundle for replay and diagnostics by trace_id.",
	),
	ToolDefinition::new(
		"elf_admin_events_ingestion_profiles_list",
		HttpMethod::Get,
		"/v2/admin/events/ingestion-profiles",
		"List latest ingestion profiles for add_event.",
	),
	ToolDefinition::new(
		"elf_admin_events_ingestion_profiles_create",
		HttpMethod::Post,
		"/v2/admin/events/ingestion-profiles",
		"Create a new ingestion profile version for add_event.",
	),
	ToolDefinition::new(
		"elf_admin_events_ingestion_profile_get",
		HttpMethod::Get,
		"/v2/admin/events/ingestion-profiles/{profile_id}",
		"Get a single ingestion profile by id/version for add_event.",
	),
	ToolDefinition::new(
		"elf_admin_events_ingestion_profile_versions_list",
		HttpMethod::Get,
		"/v2/admin/events/ingestion-profiles/{profile_id}/versions",
		"List all versions of one ingestion profile for add_event.",
	),
	ToolDefinition::new(
		"elf_admin_events_ingestion_profile_default_get",
		HttpMethod::Get,
		"/v2/admin/events/ingestion-profiles/default",
		"Get the active default ingestion profile for add_event.",
	),
	ToolDefinition::new(
		"elf_admin_events_ingestion_profile_default_set",
		HttpMethod::Put,
		"/v2/admin/events/ingestion-profiles/default",
		"Set the default ingestion profile for add_event.",
	),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ToolDefinition {
	name: &'static str,
	method: HttpMethod,
	path: &'static str,
	description: &'static str,
	streaming: bool,
}
impl ToolDefinition {
	const fn new(
		name: &'static str,
		method: HttpMethod,
		path: &'static str,
		description: &'static str,
	) -> Self {
		Self { name, method, path, description, streaming: true }
	}
}

fn build_tools() -> HashMap<&'static str, ToolDefinition> {
	ALL_TOOL_DEFINITIONS.into_iter().map(|tool| (tool.name, tool)).collect()
}

#[test]
fn registers_all_tools() {
	let tools = build_tools();
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

#[test]
fn recall_debug_tool_uses_public_agent_route() {
	let tools = build_tools();
	let tool = tools.get("elf_recall_debug_panel").expect("Missing recall debug panel tool.");

	assert_eq!(tool.path, "/v2/recall-debug/panel");
	assert!(tool.description.contains("recall_trace"));
}

#[test]
fn searches_notes_tool_description_mentions_payload_level_shapes() {
	let tools = build_tools();
	let tool =
		tools.get("elf_searches_notes").expect("Missing elf_searches_notes tool definition.");
	let description = tool.description.to_lowercase();

	assert_eq!(tool.path, "/v2/searches/{search_id}/notes");
	assert!(description.contains("l0"));
	assert!(description.contains("l1"));
	assert!(description.contains("l2"));
	assert!(description.contains("source_ref"));
	assert!(description.contains("structured"));
}
