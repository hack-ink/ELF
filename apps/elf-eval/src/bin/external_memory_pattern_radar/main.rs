#![allow(unused_crate_dependencies)]

//! Weekly external memory pattern radar runner.

mod cli;
mod decision;
mod github;
mod io;
mod render;
mod runtime;
mod types;
mod validation;

use clap::Parser;
use color_eyre::Result;

use self::cli::{Args, Command};

const CURSOR_SCHEMA: &str = "elf.external_memory_pattern_radar_cursor/v1";
const RUN_SCHEMA: &str = "elf.external_memory_pattern_radar_run/v1";
const DEFAULT_CURSOR: &str = "apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json";
const DEFAULT_SUMMARY: &str = "docs/evidence/external_memory_pattern_radar_latest.md";

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		Command::Run(args) => self::runtime::run_radar(args).await,
		Command::Validate(args) => self::validation::validate_command(&args.cursor),
	}
}
