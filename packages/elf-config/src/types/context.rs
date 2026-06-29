use std::collections::HashMap;

use serde::Deserialize;

/// Optional metadata used to improve retrieval disambiguation across projects and scopes.
#[derive(Debug, Deserialize)]
pub struct Context {
	/// Optional. Map keys are either "<tenant_id>:<project_id>" or "<project_id>".
	pub project_descriptions: Option<HashMap<String, String>>,
	/// Optional. Map keys are scope labels, e.g. "project_shared".
	pub scope_descriptions: Option<HashMap<String, String>>,
	/// Optional. Additive boost applied to final scores when a query's tokens match a scope
	/// description.
	pub scope_boost_weight: Option<f32>,
}

/// Static forwarding context attached by `elf-mcp` to proxied requests.
#[derive(Clone, Debug, Deserialize)]
pub struct McpContext {
	/// Tenant identifier attached to proxied MCP requests.
	pub tenant_id: String,
	/// Project identifier attached to proxied MCP requests.
	pub project_id: String,
	/// Agent identifier attached to proxied MCP requests.
	pub agent_id: String,
	/// Read profile attached to proxied MCP requests.
	pub read_profile: String,
}
