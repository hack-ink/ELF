#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Offline adapter for agentmemory-style fixture exports.

#[path = "agentmemory_fixture_adapter/adapt.rs"] mod adapt;
#[path = "agentmemory_fixture_adapter/cli.rs"] mod cli;
#[path = "agentmemory_fixture_adapter/io.rs"] mod io;
#[path = "agentmemory_fixture_adapter/mapping.rs"] mod mapping;
#[path = "agentmemory_fixture_adapter/types.rs"] mod types;
#[path = "agentmemory_fixture_adapter/util.rs"] mod util;

use clap::Parser;

use self::{
	adapt::adapt_fixture,
	cli::Args,
	io::{read_fixture, write_output},
};

const OUTPUT_SCHEMA: &str = "elf.agentmemory_adapter/v1";
const FIXTURE_RESOLVER: &str = "agentmemory_fixture/v1";
const DEFAULT_IMPORTANCE: f32 = 0.5;
const DEFAULT_CONFIDENCE: f32 = 0.5;

fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let fixture = read_fixture(&args.fixture)?;
	let output = adapt_fixture(&fixture, args.scope.as_str(), args.max_note_chars);
	let json = serde_json::to_string_pretty(&output)?;

	if let Some(path) = args.out {
		write_output(path, json.as_str())?;
	} else {
		println!("{json}");
	}

	Ok(())
}
