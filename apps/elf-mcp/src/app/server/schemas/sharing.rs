use std::sync::Arc;

use rmcp::model::JsonObject;

pub(in crate::app::server) fn space_grants_list_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["space"],
		"properties": {
			"space": { "type": "string", "enum": ["team_shared", "org_shared"] }
		}
	}))
}

pub(in crate::app::server) fn space_grant_upsert_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["space", "grantee_kind"],
		"properties": {
			"space": { "type": "string", "enum": ["team_shared", "org_shared"] },
			"grantee_kind": { "type": "string", "enum": ["project", "agent"] },
			"grantee_agent_id": { "type": ["string", "null"] }
		}
	}))
}

pub(in crate::app::server) fn space_grant_revoke_schema() -> Arc<JsonObject> {
	space_grant_upsert_schema()
}
