use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn core_blocks_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"properties": {
			"read_profile": { "type": ["string", "null"] }
		}
	}))
}

pub(in crate::app::server) fn entity_memory_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"properties": {
			"entity_id": { "type": ["string", "null"], "format": "uuid" },
			"entity_surface": { "type": ["string", "null"] },
			"read_profile": { "type": ["string", "null"] }
		}
	}))
}

pub(in crate::app::server) fn dreaming_review_queue_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"properties": {
			"run_id": { "type": ["string", "null"], "format": "uuid" },
			"review_state": {
				"type": ["string", "null"],
				"enum": ["proposed", "approved", "rejected", "applied", "archived", null]
			},
			"limit": {
				"type": ["integer", "null"],
				"minimum": 1,
				"maximum": 200
			}
		}
	}))
}

pub(in crate::app::server) fn recall_debug_panel_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": false,
		"properties": {
			"trace_id": { "type": ["string", "null"], "format": "uuid" },
			"query": { "type": ["string", "null"] },
			"docs_query": { "type": ["string", "null"] },
			"knowledge_query": { "type": ["string", "null"] },
			"graph_subject": {
				"oneOf": [
					{
						"type": "object",
						"additionalProperties": false,
						"required": ["entity_id"],
						"properties": {
							"entity_id": {
								"type": "string",
								"format": "uuid"
							}
						}
					},
					{
						"type": "object",
						"additionalProperties": false,
						"required": ["surface"],
						"properties": {
							"surface": { "type": "string" }
						}
					},
					{ "type": "null" }
				]
			},
			"graph_predicate": {
				"oneOf": [
					{
						"type": "object",
						"additionalProperties": false,
						"required": ["predicate_id"],
						"properties": {
							"predicate_id": {
								"type": "string",
								"format": "uuid"
							}
						}
					},
					{
						"type": "object",
						"additionalProperties": false,
						"required": ["surface"],
						"properties": {
							"surface": { "type": "string" }
						}
					},
					{ "type": "null" }
				]
			},
			"include_dreaming": { "type": ["boolean", "null"] },
			"limit": {
				"type": ["integer", "null"],
				"minimum": 1,
				"maximum": 100
			}
		}
	}))
}
