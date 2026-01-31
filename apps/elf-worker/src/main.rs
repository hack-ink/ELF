use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	let args = Args::parse();
	let _config = elf_config::load(&args.config)?;
	Ok(())
}
