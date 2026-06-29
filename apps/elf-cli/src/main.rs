//! Local ELF CLI wrappers for production memory workflows.

mod args;
mod commands;
mod diagnostics;
mod http;
mod json;
mod tasks;

use clap::Parser;
use color_eyre::Result;
use reqwest::Client;

use crate::args::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	run(Cli::parse()).await
}

async fn run(cli: Cli) -> Result<()> {
	let client = Client::new();

	match cli.command {
		Commands::AddNote(args) => commands::run_add_note(&client, args).await,
		Commands::Search(args) => commands::run_search(&client, args).await,
		Commands::Status(args) => commands::run_status(&client, args).await,
		Commands::Backfill(args) => tasks::run_backfill(args),
		Commands::Benchmark(args) => tasks::run_benchmark(args),
		Commands::Diagnostics(args) => diagnostics::run_diagnostics(&client, args).await,
	}
}
