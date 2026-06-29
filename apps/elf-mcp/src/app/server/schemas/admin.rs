use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn admin_traces_recent_list_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": [],
		"properties": {
			"limit": {
				"type": ["integer", "null"],
				"minimum": 1,
				"maximum": 200
			},
			"cursor_created_at": { "type": ["string", "null"], "format": "date-time" },
			"cursor_trace_id": { "type": ["string", "null"] },
			"agent_id": { "type": ["string", "null"] },
			"read_profile": { "type": ["string", "null"] },
			"created_after": { "type": ["string", "null"], "format": "date-time" },
			"created_before": { "type": ["string", "null"], "format": "date-time" }
		}
	}))
}

pub(in crate::app::server) fn admin_trace_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["trace_id"],
		"properties": {
			"trace_id": { "type": "string" }
		}
	}))
}

pub(in crate::app::server) fn admin_trajectory_get_schema() -> Arc<JsonObject> {
	admin_trace_get_schema()
}

pub(in crate::app::server) fn admin_trace_item_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["item_id"],
		"properties": {
			"item_id": { "type": "string" }
		}
	}))
}

pub(in crate::app::server) fn admin_note_provenance_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["note_id"],
		"properties": {
			"note_id": { "type": "string" }
		}
	}))
}

pub(in crate::app::server) fn admin_memory_history_get_schema() -> Arc<JsonObject> {
	admin_note_provenance_get_schema()
}

pub(in crate::app::server) fn admin_trace_bundle_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["trace_id"],
		"properties": {
			"trace_id": { "type": "string" },
			"mode": { "type": ["string", "null"], "enum": ["bounded", "full", null] },
			"stage_items_limit": {
				"type": ["integer", "null"],
				"minimum": 0,
				"maximum": 256
			},
			"candidates_limit": {
				"type": ["integer", "null"],
				"minimum": 0,
				"maximum": 1_000
			}
		}
	}))
}

pub(in crate::app::server) fn admin_ingestion_profiles_list_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": [],
		"properties": {}
	}))
}

pub(in crate::app::server) fn admin_ingestion_profiles_create_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["profile_id", "profile", "created_by"],
		"properties": {
			"profile_id": { "type": "string" },
			"version": { "type": ["integer", "null"] },
			"profile": { "type": "object", "additionalProperties": true },
			"created_by": { "type": "string" },
		}
	}))
}

pub(in crate::app::server) fn admin_ingestion_profile_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["profile_id"],
		"properties": {
			"profile_id": { "type": "string" },
			"version": { "type": ["integer", "null"] },
		}
	}))
}

pub(in crate::app::server) fn admin_ingestion_profile_versions_list_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["profile_id"],
		"properties": {
			"profile_id": { "type": "string" }
		}
	}))
}

pub(in crate::app::server) fn admin_ingestion_profile_default_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": [],
		"properties": {}
	}))
}

pub(in crate::app::server) fn admin_ingestion_profile_default_set_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["profile_id"],
		"properties": {
			"profile_id": { "type": "string" },
			"version": { "type": ["integer", "null"] },
		}
	}))
}
