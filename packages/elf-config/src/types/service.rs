use serde::Deserialize;

/// Bind addresses and logging settings for ELF services.
#[derive(Debug, Deserialize)]
pub struct Service {
	/// Bind address for the public HTTP API.
	pub http_bind: String,
	/// Bind address for the MCP server entrypoint.
	pub mcp_bind: String,
	/// Bind address for the admin HTTP API.
	pub admin_bind: String,
	/// Default service log level.
	pub log_level: String,
}
