use std::path::PathBuf;

use clap::{Parser, Subcommand};

use super::{DEFAULT_CURSOR, DEFAULT_SUMMARY, types::RadarMode};

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub(super) struct Args {
	#[command(subcommand)]
	pub(super) command: Command,
}

#[derive(Debug, Parser)]
pub(super) struct RunArgs {
	/// Existing radar cursor file.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_CURSOR)]
	pub(super) cursor: PathBuf,
	/// Output cursor path. Defaults to updating --cursor.
	#[arg(long, value_name = "FILE")]
	pub(super) out_cursor: Option<PathBuf>,
	/// Output Markdown summary path.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_SUMMARY)]
	pub(super) summary: PathBuf,
	/// Observation mode. Use offline for deterministic dry runs.
	#[arg(long, value_enum, default_value_t = RadarMode::Live)]
	pub(super) mode: RadarMode,
	/// Stable run id. Defaults to external-memory-pattern-radar-YYYY-MM-DD.
	#[arg(long)]
	pub(super) run_id: Option<String>,
	/// Environment variable containing a GitHub token for live mode.
	#[arg(long, default_value = "GITHUB_TOKEN")]
	pub(super) github_token_env: String,
}

#[derive(Debug, Parser)]
pub(super) struct ValidateArgs {
	/// Cursor file to validate.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_CURSOR)]
	pub(super) cursor: PathBuf,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
pub(super) enum Command {
	/// Run the external memory radar and write cursor plus Markdown summary.
	Run(RunArgs),
	/// Validate a radar cursor and its latest decision records.
	Validate(ValidateArgs),
}
