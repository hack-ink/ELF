use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
	time::Duration,
};

use axum::{
	Json, Router,
	extract::State,
	http::{HeaderMap, Method, Uri},
	routing,
};
use serde_json::Map;
use tokio::{
	net::TcpListener,
	sync::{
		oneshot,
		oneshot::{Receiver, Sender},
	},
	time,
};

use crate::app::{
	McpAuthState,
	server::{ElfContextHeaders, ElfMcp, HEADER_AUTHORIZATION, HttpMethod},
};
use elf_config::McpContext;

type RequestRecorder = Arc<Mutex<Option<Sender<RecordedRequest>>>>;

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

struct RecordedRequest {
	method: Method,
	path: String,
	body: serde_json::Value,
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
fn notes_ingest_schema_includes_structured_entities_relations() {
	let schema = super::notes_ingest_schema();
	let notes = schema
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("notes ingest schema is missing properties.")
		.get("notes")
		.and_then(serde_json::Value::as_object)
		.expect("notes schema is missing notes.");
	let note_items = notes
		.get("items")
		.and_then(serde_json::Value::as_object)
		.expect("notes schema is missing items.");
	let note_properties = note_items
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("notes schema is missing note item properties.");
	let structured = note_properties
		.get("structured")
		.and_then(serde_json::Value::as_object)
		.expect("notes schema is missing structured.");
	let structured_type = structured
		.get("type")
		.and_then(serde_json::Value::as_array)
		.expect("structured.type is not an array.");

	assert!(
		structured_type.contains(&serde_json::Value::String("object".to_string()))
			&& structured_type.contains(&serde_json::Value::String("null".to_string()))
	);

	let structured_properties = structured
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("structured schema is missing properties.");

	assert!(structured_properties.contains_key("entities"));
	assert!(structured_properties.contains_key("relations"));

	let relation_object = structured_properties
		.get("relations")
		.and_then(serde_json::Value::as_object)
		.and_then(|relations| relations.get("items"))
		.and_then(serde_json::Value::as_object)
		.and_then(|items| items.get("properties"))
		.and_then(serde_json::Value::as_object)
		.expect("relations schema is missing properties.")
		.get("object")
		.and_then(serde_json::Value::as_object)
		.expect("relation schema is missing object.");
	let one_of = relation_object
		.get("oneOf")
		.and_then(serde_json::Value::as_array)
		.expect("relation object is missing oneOf.");

	assert_eq!(one_of.len(), 2, "relation object should have entity/value oneOf variants.");
	assert!(one_of.iter().any(|variant| {
		variant.as_object().is_some_and(|branch| {
			branch
				.get("required")
				.and_then(serde_json::Value::as_array)
				.is_some_and(|required| required.iter().any(|value| value == "entity"))
		})
	}));
	assert!(one_of.iter().any(|variant| {
		variant.as_object().is_some_and(|branch| {
			branch
				.get("required")
				.and_then(serde_json::Value::as_array)
				.is_some_and(|required| required.iter().any(|value| value == "value"))
		})
	}));
}

#[test]
fn admin_paths_use_admin_api_base() {
	let context = McpContext {
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		read_profile: "private_plus_project".to_string(),
	};
	let mcp = ElfMcp::new(
		"http://127.0.0.1:9000".to_string(),
		"http://127.0.0.1:9001".to_string(),
		ElfContextHeaders::new(&context),
		McpAuthState::Off,
	);

	assert_eq!(mcp.api_base_for_path("/v2/admin/traces/recent"), "http://127.0.0.1:9001");
	assert_eq!(mcp.api_base_for_path("/v2/admin/notes/abcd/provenance"), "http://127.0.0.1:9001");
	assert_eq!(mcp.api_base_for_path("/v2/admin/notes/abcd/history"), "http://127.0.0.1:9001");
	assert_eq!(mcp.api_base_for_path("/v2/searches"), "http://127.0.0.1:9000");
	assert_eq!(mcp.api_base_for_path("/v2/recall-debug/panel"), "http://127.0.0.1:9000");
}

#[test]
fn recall_debug_tool_uses_public_agent_route() {
	let tools = build_tools();
	let tool = tools.get("elf_recall_debug_panel").expect("Missing recall debug panel tool.");

	assert_eq!(tool.path, "/v2/recall-debug/panel");
	assert!(tool.description.contains("recall_trace"));
}

#[test]
fn recall_debug_panel_schema_rejects_context_override_fields() {
	let schema = super::recall_debug_panel_schema();
	let properties = schema
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("recall debug panel schema is missing properties.");

	assert_eq!(schema.get("additionalProperties"), Some(&serde_json::Value::Bool(false)));

	for key in ["tenant_id", "project_id", "agent_id", "read_profile"] {
		assert!(!properties.contains_key(key), "{key} must not be a tool param.");
	}
	for key in ["graph_subject", "graph_predicate"] {
		let one_of = properties
			.get(key)
			.and_then(serde_json::Value::as_object)
			.and_then(|schema| schema.get("oneOf"))
			.and_then(serde_json::Value::as_array)
			.expect("selector schema is missing oneOf.");

		for branch in one_of.iter().filter_map(serde_json::Value::as_object) {
			if branch.get("type").and_then(serde_json::Value::as_str) == Some("object") {
				assert_eq!(
					branch.get("additionalProperties"),
					Some(&serde_json::Value::Bool(false)),
					"{key} selector object branches must be closed."
				);
			}
		}
	}
}

#[test]
fn off_mode_allows_requests_without_auth_header() {
	let headers = HeaderMap::new();

	assert!(super::is_authorized(&headers, &McpAuthState::Off));
}

#[test]
fn static_keys_mode_requires_authorization_bearer_header() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "Bearer token-a".parse().expect("valid header"));

	assert!(super::is_authorized(
		&headers,
		&McpAuthState::StaticKeys { bearer_token: "token-a".to_string() }
	));
}

#[test]
fn static_keys_mode_rejects_non_bearer_schemes() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "bearer token-a".parse().expect("valid header"));

	assert!(!super::is_authorized(
		&headers,
		&McpAuthState::StaticKeys { bearer_token: "token-a".to_string() }
	));
}

#[test]
fn docs_search_l0_schema_includes_filter_fields() {
	let schema = super::docs_search_l0_schema();
	let properties = schema
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("docs_search_l0 schema is missing properties.");
	let required = ["query"];
	let expected = [
		"scope",
		"status",
		"doc_type",
		"agent_id",
		"thread_id",
		"updated_after",
		"updated_before",
		"ts_gte",
		"ts_lte",
		"sparse_mode",
		"domain",
		"repo",
		"explain",
	];

	for field in required {
		assert!(
			schema
				.get("required")
				.and_then(serde_json::Value::as_array)
				.is_some_and(|fields| { fields.iter().any(|value| value.as_str() == Some(field)) }),
			"Missing required field {field}."
		);
	}
	for field in expected {
		assert!(properties.contains_key(field), "Missing schema field: {field}.");
	}

	assert_eq!(
		properties.get("status").and_then(serde_json::Value::as_object).and_then(|status| {
			status.get("enum").and_then(serde_json::Value::as_array).map(|vals| vals.to_vec())
		}),
		Some(vec![
			serde_json::Value::String("active".to_string()),
			serde_json::Value::String("deleted".to_string()),
			serde_json::Value::Null,
		])
	);
	assert_eq!(
		properties.get("sparse_mode").and_then(serde_json::Value::as_object).and_then(|field| {
			field.get("enum").and_then(serde_json::Value::as_array).map(|vals| vals.to_vec())
		}),
		Some(vec![
			serde_json::Value::String("auto".to_string()),
			serde_json::Value::String("on".to_string()),
			serde_json::Value::String("off".to_string()),
			serde_json::Value::Null,
		])
	);
}

#[test]
fn docs_put_schema_includes_required_fields_and_write_policy() {
	let schema = super::docs_put_schema();
	let properties = schema
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("docs_put schema is missing properties.");
	let required = ["scope", "content", "source_ref"];
	let expected = ["scope", "doc_type", "title", "source_ref", "write_policy", "content"];

	for field in required {
		assert!(
			schema
				.get("required")
				.and_then(serde_json::Value::as_array)
				.is_some_and(|fields| { fields.iter().any(|value| value.as_str() == Some(field)) }),
			"Missing required field {field}."
		);
	}
	for field in expected {
		assert!(properties.contains_key(field), "Missing schema field: {field}.");
	}

	let write_policy = properties.get("write_policy").and_then(serde_json::Value::as_object);
	let source_ref_properties = properties
		.get("source_ref")
		.and_then(|value| value.get("properties"))
		.and_then(serde_json::Value::as_object)
		.expect("docs_put source_ref schema is missing properties.");

	assert!(
		write_policy.is_some_and(|field| {
			field.get("type").and_then(serde_json::Value::as_array).is_some_and(|types| {
				types.contains(&serde_json::Value::String("object".to_string()))
					&& types.contains(&serde_json::Value::String("null".to_string()))
			})
		}),
		"Missing write_policy object/null type in docs_put schema."
	);

	for field in ["source_kind", "canonical_uri", "captured_at", "trust_label", "excerpt_locator"] {
		assert!(source_ref_properties.contains_key(field), "Missing source_ref field: {field}.");
	}
}

#[test]
fn work_journal_schemas_include_families_and_source_refs() {
	let create_schema = super::work_journal_entry_create_schema();
	let create_properties = create_schema
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("work_journal_entry_create schema is missing properties.");
	let readback_schema = super::work_journal_session_readback_schema();
	let readback_properties = readback_schema
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("work_journal_session_readback schema is missing properties.");

	for field in ["scope", "session_id", "family", "body", "source_refs"] {
		assert!(
			create_schema
				.get("required")
				.and_then(serde_json::Value::as_array)
				.is_some_and(|fields| { fields.iter().any(|value| value.as_str() == Some(field)) }),
			"Missing Work Journal required field {field}."
		);
	}

	assert!(create_properties.contains_key("write_policy"));
	assert!(create_properties.contains_key("promotion_boundary"));
	assert!(readback_properties.contains_key("session_id"));
	assert!(readback_properties.contains_key("families"));
}

#[test]
fn docs_excerpts_get_schema_includes_l0_level_and_optional_explain() {
	let schema = super::docs_excerpts_get_schema();
	let properties = schema
		.get("properties")
		.and_then(serde_json::Value::as_object)
		.expect("docs_excerpts_get schema is missing properties.");
	let level_values = properties
		.get("level")
		.and_then(|level| level.get("enum"))
		.and_then(|values| values.as_array())
		.expect("docs_excerpts_get level schema is missing enum.");

	assert!(level_values.contains(&serde_json::Value::String("L0".to_string())));
	assert!(properties.contains_key("explain"));
}

#[test]
fn payload_level_schema_for_search_tools_is_l0_l1_l2() {
	for schema in [
		super::searches_create_schema(),
		super::searches_get_schema(),
		super::searches_timeline_schema(),
		super::searches_notes_schema(),
	] {
		let properties = schema
			.get("properties")
			.and_then(serde_json::Value::as_object)
			.expect("Search schema is missing properties.");
		let payload_level = properties
			.get("payload_level")
			.and_then(serde_json::Value::as_object)
			.expect("payload_level field is missing from search schema.");
		let payload_level_values = payload_level
			.get("enum")
			.and_then(serde_json::Value::as_array)
			.expect("payload_level enum is missing.");

		assert_eq!(payload_level_values.len(), 4, "Unexpected payload_level enum length.");
		assert!(payload_level_values.iter().any(|value| value.as_str() == Some("l0")));
		assert!(payload_level_values.iter().any(|value| value.as_str() == Some("l1")));
		assert!(payload_level_values.iter().any(|value| value.as_str() == Some("l2")));
		assert!(payload_level_values.iter().any(|value| value.is_null()));
	}
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

#[tokio::test]
async fn recall_debug_panel_rejects_context_override_params() {
	let context = McpContext {
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		read_profile: "private_plus_project".to_string(),
	};
	let mcp = ElfMcp::new(
		"http://127.0.0.1:1".to_string(),
		"http://127.0.0.1:1".to_string(),
		ElfContextHeaders::new(&context),
		McpAuthState::Off,
	);
	let params = Map::from_iter([(
		"tenant_id".to_string(),
		serde_json::Value::String("tenant-override".to_string()),
	)]);
	let result = mcp.elf_recall_debug_panel(params).await;
	let err = result.expect_err("context override params must fail before forwarding.");

	assert!(format!("{err:?}").contains("tenant_id"));
}

#[tokio::test]
async fn default_ingestion_profile_set_uses_put_admin_default_path() {
	let (admin_base, received) = spawn_recording_admin_server().await;
	let context = McpContext {
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		read_profile: "private_plus_project".to_string(),
	};
	let mcp = ElfMcp::new(
		"http://127.0.0.1:9000".to_string(),
		admin_base,
		ElfContextHeaders::new(&context),
		McpAuthState::Off,
	);
	let params = Map::from_iter([
		("profile_id".to_string(), serde_json::Value::String("profile-a".to_string())),
		("version".to_string(), serde_json::Value::Number(2.into())),
	]);
	let result = mcp.elf_admin_events_ingestion_profile_default_set(params).await;

	assert!(result.is_ok(), "default setter should forward successfully: {result:?}");

	let request = receive_recorded_request(received).await;

	assert_eq!(request.method, Method::PUT);
	assert_eq!(request.path, "/v2/admin/events/ingestion-profiles/default");
	assert_eq!(
		request.body.get("profile_id").and_then(serde_json::Value::as_str),
		Some("profile-a")
	);
	assert_eq!(request.body.get("version").and_then(serde_json::Value::as_i64), Some(2));
}

async fn spawn_recording_admin_server() -> (String, Receiver<RecordedRequest>) {
	let (tx, rx) = oneshot::channel();
	let app = Router::new()
		.route("/v2/admin/events/ingestion-profiles/default", routing::any(record_request))
		.with_state(Arc::new(Mutex::new(Some(tx))));
	let listener = match TcpListener::bind("127.0.0.1:0").await {
		Ok(listener) => listener,
		Err(err) => panic!("Failed to bind MCP recording admin server: {err}."),
	};
	let addr = match listener.local_addr() {
		Ok(addr) => addr,
		Err(err) => panic!("Failed to read MCP recording admin server address: {err}."),
	};

	tokio::spawn(async move {
		if let Err(err) = axum::serve(listener, app).await {
			panic!("MCP recording admin server failed: {err}.");
		}
	});

	(format!("http://{addr}"), rx)
}

async fn record_request(
	State(recorder): State<RequestRecorder>,
	method: Method,
	uri: Uri,
	Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
	let mut sender = match recorder.lock() {
		Ok(sender) => sender,
		Err(err) => panic!("MCP recording admin server mutex was poisoned: {err}."),
	};

	if let Some(tx) = sender.take() {
		let _ = tx.send(RecordedRequest { method, path: uri.path().to_string(), body });
	}

	Json(serde_json::json!({ "ok": true }))
}

async fn receive_recorded_request(received: Receiver<RecordedRequest>) -> RecordedRequest {
	match time::timeout(Duration::from_secs(3), received).await {
		Ok(Ok(request)) => request,
		Ok(Err(err)) => panic!("MCP recording admin server closed before recording: {err}."),
		Err(err) => panic!("Timed out waiting for MCP recording admin server: {err}."),
	}
}
