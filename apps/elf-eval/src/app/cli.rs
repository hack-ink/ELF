use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub struct Args {
	#[arg(long = "config-a", short = 'c', value_name = "FILE", visible_alias = "config")]
	pub config_a: PathBuf,
	#[arg(long = "config-b", value_name = "FILE")]
	pub config_b: Option<PathBuf>,
	#[arg(long, short = 'd', value_name = "FILE", required_unless_present = "trace_id")]
	pub dataset: Option<PathBuf>,
	#[arg(long, value_name = "N")]
	pub top_k: Option<u32>,
	#[arg(long, value_name = "N")]
	pub candidate_k: Option<u32>,
	#[arg(long, value_name = "N", default_value_t = 1)]
	pub runs_per_query: u32,
	#[arg(long, value_enum, default_value_t = SearchMode::PlannedSearch)]
	pub search_mode: SearchMode,
	#[arg(long = "search-mode-b", value_enum)]
	pub search_mode_b: Option<SearchMode>,
	#[arg(long = "trace-id", value_name = "UUID", num_args = 1..)]
	pub trace_id: Vec<Uuid>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
	#[value(name = "quick_find")]
	QuickFind,
	#[value(name = "planned_search")]
	PlannedSearch,
}
