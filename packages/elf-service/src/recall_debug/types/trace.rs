use serde::Serialize;
use serde_json::Value;

/// Deterministic flat recall trace over all requested layers.
#[derive(Clone, Debug, Serialize)]
pub struct RecallTrace {
	/// Trace schema identifier.
	pub schema: String,
	/// Aggregate trace counters.
	pub summary: RecallTraceSummary,
	/// Stable trace entries in layer and row order.
	pub entries: Vec<RecallTraceEntry>,
}

/// Aggregate counters for a recall trace.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RecallTraceSummary {
	/// Number of trace entries.
	pub entry_count: usize,
	/// Entries whose row selection state is selected.
	pub selected_count: usize,
	/// Entries whose row selection state is dropped.
	pub dropped_count: usize,
	/// Entries whose freshness state indicates stale or non-current evidence.
	pub stale_count: usize,
	/// Entries representing blocked layers.
	pub blocked_count: usize,
	/// Entries representing layers that were not requested.
	pub not_requested_count: usize,
	/// Entries that require raw SQL for diagnosis.
	pub raw_sql_needed_count: usize,
	/// Entries with a replay command or deterministic artifact path.
	pub replay_command_count: usize,
}

/// One compact recall trace entry.
#[derive(Clone, Debug, Serialize)]
pub struct RecallTraceEntry {
	/// Layer identifier.
	pub layer: String,
	/// Primary trace state for compact assertions.
	pub context_state: String,
	/// Original row selection state or layer evidence class.
	pub selection_state: String,
	/// Authority layer that owns the context.
	pub authority_layer: String,
	/// Freshness or temporal state.
	pub freshness_state: String,
	/// Stable identifiers for replay or hydration.
	pub item_ref: Value,
	/// Source refs or source snapshots supporting the context.
	pub source_refs: Value,
	/// Optional score.
	pub score: Option<f32>,
	/// Optional rank.
	pub rank: Option<u32>,
	/// Compact policy or stage reason for the state.
	pub policy_reason: Option<String>,
	/// Replay command or deterministic artifact path.
	pub replay_command: Option<String>,
	/// Layer or row evidence class.
	pub evidence_class: String,
	/// Whether raw SQL is required to diagnose this entry.
	pub raw_sql_needed: bool,
}
