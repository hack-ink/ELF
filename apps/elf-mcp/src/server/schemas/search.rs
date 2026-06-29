use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn searches_create_schema() -> Arc<JsonObject> {
	let filter_schema = rmcp::object!({
		"type": "object",
		"required": ["schema", "expr"],
		"properties": {
			"schema": {
				"type": "string",
				"const": "search_filter_expr/v1",
			},
			"expr": {
				"type": "object",
				"additionalProperties": true,
			},
		},
		"additionalProperties": true,
	});

	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["query", "mode"],
		"properties": {
			"query": { "type": "string" },
			"mode": { "type": "string", "enum": ["quick_find", "planned_search"] },
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
			"top_k": { "type": ["integer", "null"] },
			"candidate_k": { "type": ["integer", "null"] },
			"filter": filter_schema,
			"read_profile": { "type": ["string", "null"] }
		}
	}))
}

pub(in crate::app::server) fn searches_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["search_id"],
		"properties": {
			"search_id": { "type": "string" },
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
			"top_k": { "type": ["integer", "null"] },
			"touch": { "type": ["boolean", "null"] }
		}
	}))
}

pub(in crate::app::server) fn searches_timeline_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["search_id"],
		"properties": {
			"search_id": { "type": "string" },
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
			"group_by": { "type": ["string", "null"] }
		}
	}))
}

pub(in crate::app::server) fn searches_notes_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["search_id", "note_ids"],
		"properties": {
			"search_id": { "type": "string" },
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
			"note_ids": { "type": "array", "items": { "type": "string" } },
			"record_hits": { "type": ["boolean", "null"] }
		}
	}))
}
