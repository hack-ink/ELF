use clap::{Args, ValueEnum};

use crate::args::{AdminEndpointArgs, OutputArgs, PublicEndpointArgs, ReadContextArgs};

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
	#[command(flatten)]
	pub(crate) endpoint: PublicEndpointArgs,
	#[command(flatten)]
	pub(crate) read_context: ReadContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// English query string.
	#[arg(long)]
	pub(crate) query: String,
	/// Search mode to request from the service.
	#[arg(long, value_enum, default_value_t = SearchMode::QuickFind)]
	pub(crate) mode: SearchMode,
	/// Number of final items to return.
	#[arg(long)]
	pub(crate) top_k: Option<u32>,
	/// Candidate breadth before ranking.
	#[arg(long)]
	pub(crate) candidate_k: Option<u32>,
	/// Payload level requested from the service.
	#[arg(long, value_enum, default_value_t = PayloadLevel::L0)]
	pub(crate) payload_level: PayloadLevel,
	/// Optional search filter JSON object.
	#[arg(long)]
	pub(crate) filter_json: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct AdminSearchArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) read_context: ReadContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// English query string.
	#[arg(long)]
	pub(crate) query: String,
	/// Search mode to request from the service.
	#[arg(long, value_enum, default_value_t = SearchMode::QuickFind)]
	pub(crate) mode: SearchMode,
	/// Number of final items to return.
	#[arg(long)]
	pub(crate) top_k: Option<u32>,
	/// Candidate breadth before ranking.
	#[arg(long)]
	pub(crate) candidate_k: Option<u32>,
	/// Payload level requested from the service.
	#[arg(long, value_enum, default_value_t = PayloadLevel::L2)]
	pub(crate) payload_level: PayloadLevel,
	/// Optional search filter JSON object.
	#[arg(long)]
	pub(crate) filter_json: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
pub(crate) enum SearchMode {
	QuickFind,
	PlannedSearch,
}
impl SearchMode {
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::QuickFind => "quick_find",
			Self::PlannedSearch => "planned_search",
		}
	}
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lower")]
pub(crate) enum PayloadLevel {
	L0,
	L1,
	L2,
}
impl PayloadLevel {
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::L0 => "l0",
			Self::L1 => "l1",
			Self::L2 => "l2",
		}
	}
}
