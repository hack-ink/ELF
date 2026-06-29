mod runtime;
mod schemas;
mod state;
mod support;
mod tools;

pub use runtime::serve_mcp;

use rmcp::handler::server::router::tool::ToolRouter;

#[cfg(test)]
use schemas::{
	docs_excerpts_get_schema, docs_put_schema, docs_search_l0_schema, notes_ingest_schema,
	recall_debug_panel_schema, searches_create_schema, searches_get_schema, searches_notes_schema,
	searches_timeline_schema, work_journal_entry_create_schema,
	work_journal_session_readback_schema,
};
use state::{ElfContextHeaders, ElfMcp, HttpMethod};
#[cfg(test)] use support::is_authorized;
use support::{
	handle_response, is_admin_path, mcp_auth_middleware, normalize_api_base, params_to_query,
};

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const HEADER_REQUEST_ID: &str = "X-ELF-Request-Id";
const HEADER_AUTHORIZATION: &str = "Authorization";

impl ElfMcp {
	pub(in crate::app::server) fn tool_router() -> ToolRouter<Self> {
		Self::core_tool_router() + Self::docs_tool_router() + Self::admin_tool_router()
	}
}

#[cfg(test)] mod tests;
