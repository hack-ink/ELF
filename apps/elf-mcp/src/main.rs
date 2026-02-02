use clap::Parser;

mod server;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	let args = Args::parse();
	let config = elf_config::load(&args.config)?;
	server::serve_mcp(&config.service.mcp_bind, &config.service.http_bind).await
}
