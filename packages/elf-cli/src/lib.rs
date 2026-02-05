use clap::builder::{
	Styles,
	styling::{AnsiColor, Effects},
};

pub const VERSION: &str = concat!(
	env!("CARGO_PKG_VERSION"),
	"-",
	env!("VERGEN_GIT_SHA"),
	"-",
	env!("VERGEN_CARGO_TARGET_TRIPLE"),
);

pub fn styles() -> Styles {
	Styles::styled()
		.header(AnsiColor::Red.on_default() | Effects::BOLD)
		.usage(AnsiColor::Red.on_default() | Effects::BOLD)
		.literal(AnsiColor::Blue.on_default() | Effects::BOLD)
		.placeholder(AnsiColor::Green.on_default())
}
