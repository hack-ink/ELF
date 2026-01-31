//! <DESCRIPTION>

#![deny(clippy::all, missing_docs, unused_crate_dependencies)]

mod cli;
mod prelude {
	pub use color_eyre::{Result, eyre};
}

use cli::Cli;
use prelude::*;

// std
use std::{panic, process};
// crates.io
use clap::Parser;
use directories::ProjectDirs;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
	color_eyre::install()?;

	let project_dirs = ProjectDirs::from("", "hack.ink", "<NAME>")
		.ok_or_else(|| eyre::eyre!("Failed to resolve project directories."))?;
	let app_root = project_dirs.data_dir();
	let (non_blocking, _guard) = tracing_appender::non_blocking(
		RollingFileAppender::builder()
			.rotation(Rotation::WEEKLY)
			.max_log_files(3)
			.filename_suffix("log")
			.build(app_root)?,
	);
	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

	tracing_subscriber::fmt()
		.with_env_filter(filter)
		.with_ansi(false)
		.with_writer(non_blocking)
		.init();

	let default_hook = panic::take_hook();

	panic::set_hook(Box::new(move |p| {
		default_hook(p);

		process::abort();
	}));

	Cli::parse().run()?;

	Ok(())
}
