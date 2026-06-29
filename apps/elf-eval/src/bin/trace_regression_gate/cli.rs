use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub(super) struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	pub(super) config: PathBuf,
	#[arg(long, short = 'g', value_name = "FILE")]
	pub(super) gate: PathBuf,
	#[arg(long, value_name = "FILE")]
	pub(super) out: Option<PathBuf>,
	#[arg(long, value_name = "N")]
	pub(super) top_k: Option<u32>,
	#[arg(long, value_name = "N")]
	pub(super) retrieval_retention_rank: Option<u32>,
}
