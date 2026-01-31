// crates.io
use clap::{
	Parser,
	builder::{
		Styles,
		styling::{AnsiColor, Effects},
	},
};
// self
use crate::prelude::*;

/// Cli.
#[derive(Debug, Parser)]
#[command(
	version = concat!(
		env!("CARGO_PKG_VERSION"),
		"-",
		env!("VERGEN_GIT_SHA"),
		"-",
		env!("VERGEN_CARGO_TARGET_TRIPLE"),
	),
	rename_all = "kebab",
	styles = styles(),
)]
pub struct Cli {
	/// Placeholder.
	#[arg(long, short, value_name = "NUM", default_value_t = String::from("Welcome to use vibe-mono!"))]
	placeholder: String,
}
impl Cli {
	pub fn run(&self) -> Result<()> {
		tracing::info!("{self:?}");

		Ok(())
	}
}

fn styles() -> Styles {
	Styles::styled()
		.header(AnsiColor::Red.on_default() | Effects::BOLD)
		.usage(AnsiColor::Red.on_default() | Effects::BOLD)
		.literal(AnsiColor::Blue.on_default() | Effects::BOLD)
		.placeholder(AnsiColor::Green.on_default())
}

#[cfg(test)]
mod tests {
	// self
	use super::*;

	#[test]
	fn default_placeholder_mentions_vibe_mono() {
		assert_eq!(Cli::parse_from(["app"]).placeholder, "Welcome to use vibe-mono!");
	}
}
