use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn docs_put_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
	"type": "object",
	"additionalProperties": true,
	"required": ["scope", "content", "source_ref"],
	"properties": {
		"scope": { "type": "string", "enum": ["agent_private", "project_shared", "org_shared"] },
		"doc_type": {
			"type": ["string", "null"],
			"enum": ["knowledge", "chat", "search", "dev", null]
		},
		"title": { "type": ["string", "null"] },
		"source_ref": {
			"type": "object",
			"additionalProperties": true,
			"required": ["schema", "doc_type", "ts"],
			"properties": {
				"schema": { "type": "string", "enum": ["doc_source_ref/v1"] },
				"doc_type": {
					"type": "string",
					"enum": ["knowledge", "chat", "search", "dev"],
				},
				"ts": { "type": "string", "format": "date-time" },
				"thread_id": { "type": "string" },
				"role": { "type": "string" },
				"query": { "type": "string" },
				"url": { "type": "string" },
				"domain": { "type": "string" },
				"repo": { "type": "string" },
				"commit_sha": { "type": "string" },
				"pr_number": { "type": "integer" },
				"issue_number": { "type": "integer" },
				"source_kind": {
					"type": "string",
					"enum": ["article", "social_thread", "pdf", "text_export", "repo_file", "chat_excerpt", "web_page"]
				},
				"canonical_uri": { "type": "string" },
				"captured_at": { "type": "string", "format": "date-time" },
				"source_created_at": { "type": "string", "format": "date-time" },
				"trust_label": {
					"type": "string",
					"enum": ["trusted", "user_captured", "public_web", "third_party", "unverified"]
				},
				"author": { "type": "string" },
				"handle": { "type": "string" },
				"source_content_hash": { "type": "string" },
				"excerpt_locator": {
					"type": "object",
					"additionalProperties": true,
					"properties": {
						"quote": {
							"type": "object",
							"required": ["exact"],
							"properties": {
								"exact": { "type": "string" },
								"prefix": { "type": "string" },
								"suffix": { "type": "string" }
							}
						},
						"position": {
							"type": "object",
							"required": ["start", "end"],
							"properties": {
								"start": { "type": "integer" },
								"end": { "type": "integer" }
							}
						}
					}
				}
			},
			"allOf": [
				{
					"if": { "properties": { "doc_type": { "const": "chat" } }, "required": ["doc_type"] },
					"then": {
						"required": ["thread_id", "role"]
					}
				},
				{
					"if": { "properties": { "doc_type": { "const": "search" } }, "required": ["doc_type"] },
					"then": {
						"required": ["query", "url", "domain"]
					}
				},
				{
					"if": { "properties": { "doc_type": { "const": "dev" } }, "required": ["doc_type"] },
					"then": {
						"required": ["repo"],
						"oneOf": [
							{ "required": ["commit_sha"] },
							{ "required": ["pr_number"] },
							{ "required": ["issue_number"] }
						]
					}
				}
			]
		},
		"write_policy": { "type": ["object", "null"] },
		"content": { "type": "string" }
	},
	}))
}

pub(in crate::app::server) fn docs_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["doc_id"],
		"properties": {
			"doc_id": { "type": "string" }
		}
	}))
}

pub(in crate::app::server) fn docs_search_l0_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["query"],
		"properties": {
			"query": { "type": "string" },
			"scope": { "type": ["string", "null"], "enum": ["agent_private", "project_shared", "org_shared", null] },
			"status": { "type": ["string", "null"], "enum": ["active", "deleted", null] },
			"doc_type": {
				"type": ["string", "null"],
				"enum": ["knowledge", "chat", "search", "dev", null]
			},
			"agent_id": { "type": ["string", "null"] },
			"thread_id": { "type": ["string", "null"] },
			"updated_after": { "type": ["string", "null"], "format": "date-time" },
			"updated_before": { "type": ["string", "null"], "format": "date-time" },
			"ts_gte": { "type": ["string", "null"], "format": "date-time" },
			"ts_lte": { "type": ["string", "null"], "format": "date-time" },
			"top_k": { "type": ["integer", "null"] },
			"candidate_k": { "type": ["integer", "null"] },
			"sparse_mode": {
				"type": ["string", "null"],
				"enum": ["auto", "on", "off", null]
			},
			"domain": { "type": ["string", "null"] },
			"repo": { "type": ["string", "null"] },
			"explain": { "type": ["boolean", "null"] },
			"read_profile": { "type": ["string", "null"] }
		}
	}))
}

pub(in crate::app::server) fn docs_excerpts_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["doc_id", "level"],
		"properties": {
			"doc_id": { "type": "string" },
			"level": { "type": "string", "enum": ["L0", "L1", "L2"] },
			"explain": { "type": ["boolean", "null"] },
			"chunk_id": { "type": ["string", "null"] },
			"quote": {
				"type": ["object", "null"],
				"additionalProperties": true,
				"required": ["exact"],
				"properties": {
					"exact": { "type": "string" },
					"prefix": { "type": ["string", "null"] },
					"suffix": { "type": ["string", "null"] }
				}
			},
			"position": {
				"type": ["object", "null"],
				"additionalProperties": true,
				"required": ["start", "end"],
				"properties": {
					"start": { "type": "integer" },
					"end": { "type": "integer" }
				}
			}
		}
	}))
}
