use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn events_ingest_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["messages"],
		"properties": {
			"scope": { "type": ["string", "null"] },
			"dry_run": { "type": ["boolean", "null"] },
			"ingestion_profile": {
				"type": "object",
				"additionalProperties": true,
				"required": ["id"],
				"properties": {
					"id": { "type": "string" },
					"version": { "type": ["integer", "null"] },
				},
			},
			"messages": {
				"type": "array",
				"items": {
					"type": "object",
					"additionalProperties": true,
					"required": ["role", "content"],
					"properties": {
						"role": { "type": "string" },
						"content": { "type": "string" },
						"ts": { "type": ["string", "null"] },
						"msg_id": { "type": ["string", "null"] },
						"write_policy": { "type": ["object", "null"] }
					}
				}
			}
		}
	}))
}
