use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn graph_query_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["subject"],
		"properties": {
			"subject": {
				"oneOf": [
					{
						"type": "object",
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
						"required": ["surface"],
						"properties": {
							"surface": { "type": "string" }
						}
					}
				]
			},
			"predicate": {
				"oneOf": [
					{
						"type": "object",
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
						"required": ["surface"],
						"properties": {
							"surface": { "type": "string" }
						}
					}
				]
			},
			"scopes": {
				"type": ["array", "null"],
				"items": { "type": "string" }
			},
			"as_of": {
				"type": ["string", "null"],
				"format": "date-time"
			},
			"limit": {
				"type": ["integer", "null"],
				"minimum": 1,
				"maximum": 200
			},
			"explain": { "type": ["boolean", "null"] }
		}
	}))
}

pub(in crate::app::server) fn graph_report_schema() -> Arc<JsonObject> {
	graph_query_schema()
}
