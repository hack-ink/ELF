pub mod routes;
pub mod state;

use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use color_eyre::eyre;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	pub config: PathBuf,
}

pub async fn run(args: Args) -> color_eyre::Result<()> {
	let config = elf_config::load(&args.config)?;
	init_tracing(&config)?;
	let http_addr: SocketAddr = config.service.http_bind.parse()?;
	let admin_addr: SocketAddr = config.service.admin_bind.parse()?;
	if config.security.bind_localhost_only && !http_addr.ip().is_loopback() {
		return Err(eyre::eyre!(
			"http_bind must be a loopback address when bind_localhost_only is true."
		));
	}
	if !admin_addr.ip().is_loopback() {
		return Err(eyre::eyre!("admin_bind must be a loopback address."));
	}
	let state = AppState::new(config).await?;
	let app = routes::router(state.clone());
	let admin_app = routes::admin_router(state);

	let http_listener = TcpListener::bind(http_addr).await?;
	tracing::info!(%http_addr, "HTTP server listening.");
	let http_server = axum::serve(http_listener, app);

	let admin_listener = TcpListener::bind(admin_addr).await?;
	tracing::info!(%admin_addr, "Admin server listening.");
	let admin_server = axum::serve(admin_listener, admin_app);

	tokio::try_join!(http_server, admin_server)?;
	Ok(())
}

fn init_tracing(config: &elf_config::Config) -> color_eyre::Result<()> {
	let filter =
		EnvFilter::try_new(&config.service.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
	tracing_subscriber::fmt().with_env_filter(filter).init();
	Ok(())
}
