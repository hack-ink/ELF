use clap::Parser;

use elf_mcp::Args;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	let args = Args::parse();
	elf_mcp::run(args).await
}
