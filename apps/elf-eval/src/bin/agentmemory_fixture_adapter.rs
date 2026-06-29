#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Offline adapter for agentmemory-style fixture exports.

#[path = "agentmemory_fixture_adapter/adapt.rs"] mod adapt;
#[path = "agentmemory_fixture_adapter/cli.rs"] mod cli;
#[path = "agentmemory_fixture_adapter/io.rs"] mod io;
#[path = "agentmemory_fixture_adapter/mapping.rs"] mod mapping;
#[path = "agentmemory_fixture_adapter/types.rs"] mod types;
#[path = "agentmemory_fixture_adapter/util.rs"] mod util;

use clap::Parser;
use color_eyre::Result;

use self::cli::Args;

const OUTPUT_SCHEMA: &str = "elf.agentmemory_adapter/v1";
const FIXTURE_RESOLVER: &str = "agentmemory_fixture/v1";
const DEFAULT_IMPORTANCE: f32 = 0.5;
const DEFAULT_CONFIDENCE: f32 = 0.5;

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let fixture = self::io::read_fixture(&args.fixture)?;
	let output = self::adapt::adapt_fixture(&fixture, args.scope.as_str(), args.max_note_chars);
	let json = serde_json::to_string_pretty(&output)?;

	if let Some(path) = args.out {
		self::io::write_output(path, json.as_str())?;
	} else {
		println!("{json}");
	}

	Ok(())
}
