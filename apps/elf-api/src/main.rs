use clap::Parser;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	let args = elf_api::Args::parse();
	elf_api::run(args).await
}
