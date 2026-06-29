use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn work_journal_entry_create_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["scope", "session_id", "family", "body", "source_refs"],
		"properties": {
			"entry_id": { "type": ["string", "null"] },
			"scope": { "type": "string", "enum": ["agent_private", "project_shared", "org_shared"] },
			"session_id": { "type": "string" },
			"family": {
				"type": "string",
				"enum": [
					"session_log",
					"handoff_brief",
					"janitor_report",
					"explicit_next_step",
					"inferred_next_step",
					"rejected_option"
				]
			},
			"title": { "type": ["string", "null"] },
			"body": { "type": "string" },
			"source_refs": {
				"type": "array",
				"items": { "type": "object", "additionalProperties": true },
				"minItems": 1
			},
			"write_policy": { "type": ["object", "null"] },
			"explicit_next_steps": {
				"type": "array",
				"items": { "type": "string" }
			},
			"inferred_next_steps": {
				"type": "array",
				"items": { "type": "string" }
			},
			"rejected_options": {
				"type": "array",
				"items": { "type": "string" }
			},
			"promotion_boundary": {
				"type": ["object", "null"],
				"additionalProperties": true,
				"properties": {
					"authoritative_memory_allowed": { "type": "boolean" },
					"accepted_memory_authority_ref": { "type": "object", "additionalProperties": true },
					"accepted_dreaming_review_ref": { "type": "object", "additionalProperties": true }
				}
			}
		}
	}))
}

pub(in crate::app::server) fn work_journal_entry_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["entry_id"],
		"properties": {
			"entry_id": { "type": "string" }
		}
	}))
}

pub(in crate::app::server) fn work_journal_session_readback_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["session_id"],
		"properties": {
			"session_id": { "type": "string" },
			"families": {
				"type": "array",
				"items": {
					"type": "string",
					"enum": [
						"session_log",
						"handoff_brief",
						"janitor_report",
						"explicit_next_step",
						"inferred_next_step",
						"rejected_option"
					]
				}
			},
			"limit": { "type": ["integer", "null"] },
			"read_profile": { "type": ["string", "null"] }
		}
	}))
}
