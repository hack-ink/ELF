pub mod server;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::{Result, eyre};

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

pub async fn run(args: Args) -> Result<()> {
	let config = elf_config::load(&args.config)?;
	let mcp =
		config.mcp.as_ref().ok_or_else(|| eyre::eyre!("mcp section is required for elf-mcp."))?;

	server::serve_mcp(
		&config.service.mcp_bind,
		&config.service.http_bind,
		config.security.api_auth_token.as_deref(),
		mcp,
	)
	.await
}
