// crates.io
use clap::Parser;
// self
use elf_api::Args;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	let args = Args::parse();
	elf_api::run(args).await
}
