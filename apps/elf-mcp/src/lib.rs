pub mod server;

// std
use std::path::PathBuf;

// crates.io
use clap::Parser;

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
	server::serve_mcp(&config.service.mcp_bind, &config.service.http_bind).await
}
