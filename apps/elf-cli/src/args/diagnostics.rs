use clap::{Args, Subcommand};

use crate::args::{AdminEndpointArgs, AdminSearchArgs, ContextArgs, OutputArgs};

#[derive(Debug, Args)]
pub(crate) struct DiagnosticsArgs {
	#[command(subcommand)]
	pub(crate) command: DiagnosticsCommand,
}

#[derive(Debug, Args)]
pub(crate) struct AdminPostArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct RecentTracesArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Maximum trace headers to return.
	#[arg(long)]
	pub(crate) limit: Option<u32>,
}

#[derive(Debug, Args)]
pub(crate) struct TraceBundleArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Trace id to load.
	#[arg(long)]
	pub(crate) trace_id: String,
	/// Bundle mode: bounded or full.
	#[arg(long, default_value = "bounded")]
	pub(crate) mode: String,
	/// Optional per-stage item cap.
	#[arg(long)]
	pub(crate) stage_items_limit: Option<u32>,
	/// Optional replay candidate cap.
	#[arg(long)]
	pub(crate) candidates_limit: Option<u32>,
}

#[derive(Debug, Args)]
pub(crate) struct NoteProvenanceArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Note id to inspect.
	#[arg(long)]
	pub(crate) note_id: String,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
pub(crate) enum DiagnosticsCommand {
	/// Rebuild Qdrant from Postgres vectors through the admin API.
	QdrantRebuild(AdminPostArgs),
	/// Run raw admin search and include trace/result/source_ref data.
	RawSearch(AdminSearchArgs),
	/// List recent persisted search traces.
	RecentTraces(RecentTracesArgs),
	/// Read a bounded or full trace bundle.
	TraceBundle(TraceBundleArgs),
	/// Read note provenance, ingest decisions, outbox rows, and recent traces.
	NoteProvenance(NoteProvenanceArgs),
}
