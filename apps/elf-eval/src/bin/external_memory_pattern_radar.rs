#![allow(unused_crate_dependencies)]

//! Weekly external memory pattern radar runner.

#[path = "external_memory_pattern_radar/cli.rs"] mod cli;
#[path = "external_memory_pattern_radar/decision.rs"] mod decision;
#[path = "external_memory_pattern_radar/github.rs"] mod github;
#[path = "external_memory_pattern_radar/io.rs"] mod io;
#[path = "external_memory_pattern_radar/render.rs"] mod render;
#[path = "external_memory_pattern_radar/runtime.rs"] mod runtime;
#[path = "external_memory_pattern_radar/types.rs"] mod types;
#[path = "external_memory_pattern_radar/validation.rs"] mod validation;

use clap::Parser;
use color_eyre::Result;

use self::{
	cli::{Args, Command},
	runtime::run_radar,
	validation::validate_command,
};

const CURSOR_SCHEMA: &str = "elf.external_memory_pattern_radar_cursor/v1";
const RUN_SCHEMA: &str = "elf.external_memory_pattern_radar_run/v1";
const DEFAULT_CURSOR: &str = "apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json";
const DEFAULT_SUMMARY: &str = "docs/evidence/external_memory_pattern_radar_latest.md";

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		Command::Run(args) => run_radar(args).await,
		Command::Validate(args) => validate_command(&args.cursor),
	}
}
