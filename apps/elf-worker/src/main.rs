use clap::Parser;

use elf_worker::Args;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	let args = Args::parse();
	elf_worker::run(args).await
}
