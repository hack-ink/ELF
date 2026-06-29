use std::{net::SocketAddr, sync::Arc};

use axum::{Router, middleware};
use color_eyre::Result;
use rmcp::{
	ServerHandler,
	model::{ServerCapabilities, ServerInfo},
	transport::streamable_http_server::{
		StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
	},
};
use tokio::net::TcpListener;

use crate::app::{
	McpAuthState,
	server::{self, ElfContextHeaders, ElfMcp},
};
use elf_config::McpContext;

#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for ElfMcp {
	fn get_info(&self) -> ServerInfo {
		ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
			.with_instructions("ELF MCP adapter that forwards tool calls to the ELF HTTP API.")
	}
}

pub async fn serve_mcp(
	bind_addr: &str,
	api_base: &str,
	admin_base: &str,
	auth_state: McpAuthState,
	mcp_context: &McpContext,
) -> Result<()> {
	let bind_addr: SocketAddr = bind_addr.parse()?;
	let api_base = server::normalize_api_base(api_base);
	let admin_base = server::normalize_api_base(admin_base);
	let context = ElfContextHeaders::new(mcp_context);
	let middleware_auth_state = auth_state.clone();
	let client_auth_state = auth_state.clone();
	let session_manager: Arc<LocalSessionManager> = Default::default();
	let service = StreamableHttpService::new(
		move || {
			Ok(ElfMcp::new(
				api_base.clone(),
				admin_base.clone(),
				context.clone(),
				client_auth_state.clone(),
			))
		},
		session_manager,
		StreamableHttpServerConfig::default(),
	);
	let router = Router::new()
		.fallback_service(service)
		.layer(middleware::from_fn_with_state(middleware_auth_state, server::mcp_auth_middleware));
	let listener = TcpListener::bind(bind_addr).await?;

	axum::serve(listener, router).await?;

	Ok(())
}
