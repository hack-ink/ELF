use clap::Parser;
use std::net::SocketAddr;

mod routes;
mod state;

#[derive(Debug, Parser)]
struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	let args = Args::parse();
	let config = elf_config::load(&args.config)?;
	init_tracing(&config)?;
	let state = state::AppState::new(config).await?;

	let http_addr: SocketAddr = state.service.cfg.service.http_bind.parse()?;
	let admin_addr: SocketAddr = state.service.cfg.service.admin_bind.parse()?;
	if state.service.cfg.security.bind_localhost_only && !admin_addr.ip().is_loopback() {
		return Err(color_eyre::eyre::eyre!(
			"admin_bind must be loopback when bind_localhost_only is true."
		));
	}
	let app = routes::router(state.clone());
	let admin_app = routes::admin_router(state);

	let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
	let admin_listener = tokio::net::TcpListener::bind(admin_addr).await?;

	tracing::info!(%http_addr, "HTTP server listening.");
	tracing::info!(%admin_addr, "Admin server listening.");

	let http_server = axum::serve(http_listener, app);
	let admin_server = axum::serve(admin_listener, admin_app);

	tokio::try_join!(http_server, admin_server)?;
	Ok(())
}

fn init_tracing(config: &elf_config::Config) -> color_eyre::Result<()> {
	let filter = tracing_subscriber::EnvFilter::try_new(&config.service.log_level)
		.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
	tracing_subscriber::fmt().with_env_filter(filter).init();
	Ok(())
}
