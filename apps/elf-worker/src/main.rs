use clap::Parser;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	let args = elf_worker::Args::parse();
	elf_worker::run(args).await
}
