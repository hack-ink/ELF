use clap::Args;

use crate::args::{ContextArgs, OutputArgs, PublicEndpointArgs};

#[derive(Debug, Args)]
pub(crate) struct AddNoteArgs {
	#[command(flatten)]
	pub(crate) endpoint: PublicEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Scope applied to the note.
	#[arg(long, default_value = "agent_private")]
	pub(crate) scope: String,
	/// Memory note type.
	#[arg(long = "type", default_value = "fact")]
	pub(crate) note_type: String,
	/// Optional note key used by the update resolver.
	#[arg(long)]
	pub(crate) key: Option<String>,
	/// English note text.
	#[arg(long)]
	pub(crate) text: String,
	/// Ranking importance value.
	#[arg(long, default_value_t = 0.7)]
	pub(crate) importance: f32,
	/// Ranking confidence value.
	#[arg(long, default_value_t = 0.9)]
	pub(crate) confidence: f32,
	/// Optional TTL override in days.
	#[arg(long)]
	pub(crate) ttl_days: Option<i64>,
	/// Operator-visible source id copied into source_ref.ref.source_id.
	#[arg(long)]
	pub(crate) source_id: Option<String>,
	/// Full JSON object source_ref override.
	#[arg(long)]
	pub(crate) source_ref_json: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct StatusArgs {
	#[command(flatten)]
	pub(crate) endpoint: PublicEndpointArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct BackfillArgs {
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Backfill corpus document count override.
	#[arg(long)]
	pub(crate) docs: Option<u32>,
	/// Worker concurrency override for the backfill runner.
	#[arg(long)]
	pub(crate) worker_concurrency: Option<u32>,
	/// Use the checked-in 10k operator profile task.
	#[arg(long)]
	pub(crate) ten_k: bool,
	/// Use the guarded 100k operator profile task.
	#[arg(long, conflicts_with = "ten_k")]
	pub(crate) hundred_k: bool,
	/// Set the required expensive-run guard for the 100k task.
	#[arg(long)]
	pub(crate) enable_expensive: bool,
	/// Print the resolved task and environment without running it.
	#[arg(long)]
	pub(crate) dry_run: bool,
}
