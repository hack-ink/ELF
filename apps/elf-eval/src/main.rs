use clap::Parser;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	let args = elf_eval::Args::parse();
	elf_eval::run(args).await
}
