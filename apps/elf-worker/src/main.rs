use clap::Parser;
use color_eyre::Result;

use elf_worker::Args;

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();

	elf_worker::run(args).await?;

	Ok(())
}
