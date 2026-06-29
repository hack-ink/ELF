use std::path::PathBuf;

use clap::Parser;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub(super) struct Args {
	/// Path to an ELF config file (used for Postgres DSN).
	#[arg(long, short = 'c', value_name = "FILE")]
	pub(super) config: PathBuf,
	/// One or more trace IDs to export.
	#[arg(long, value_name = "UUID", required = true)]
	pub(super) trace_id: Vec<Uuid>,
	/// Write SQL to this file (defaults to stdout).
	#[arg(long, value_name = "FILE")]
	pub(super) out: Option<PathBuf>,
	/// Include trace items (search_trace_items).
	#[arg(long, default_value_t = true)]
	pub(super) include_items: bool,
	/// Include trace stages (search_trace_stages and search_trace_stage_items).
	#[arg(long, default_value_t = false)]
	pub(super) include_stages: bool,
}
