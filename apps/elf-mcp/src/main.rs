use clap::Parser;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	let args = elf_mcp::Args::parse();
	elf_mcp::run(args).await
}
