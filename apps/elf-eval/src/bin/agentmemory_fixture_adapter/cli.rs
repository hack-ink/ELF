use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub(super) struct Args {
	/// Path to a sanitized agentmemory-style JSON fixture.
	#[arg(long, short = 'f', value_name = "FILE")]
	pub(super) fixture: PathBuf,
	/// Write adapter JSON to this file (defaults to stdout).
	#[arg(long, value_name = "FILE")]
	pub(super) out: Option<PathBuf>,
	/// ELF write scope to attach to emitted note and doc candidates.
	#[arg(long, default_value = "agent_private")]
	pub(super) scope: String,
	/// Maximum note text length accepted for note candidates.
	#[arg(long, default_value_t = 240)]
	pub(super) max_note_chars: usize,
}
