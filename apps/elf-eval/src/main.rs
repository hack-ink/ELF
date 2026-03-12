#![allow(unused_crate_dependencies)]

//! CLI entrypoint for ELF evaluation commands.

mod app;

use clap::Parser;
use color_eyre::Result;

use app::Args;

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();

	app::run(args).await
}
