use super::*;

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
	let api_base = normalize_api_base(api_base);
	let admin_base = normalize_api_base(admin_base);
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
		.layer(middleware::from_fn_with_state(middleware_auth_state, mcp_auth_middleware));
	let listener = TcpListener::bind(bind_addr).await?;

	axum::serve(listener, router).await?;

	Ok(())
}
