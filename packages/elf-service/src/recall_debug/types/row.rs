use serde::Serialize;
use serde_json::Value;

/// One item in the recall/debug panel.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugRow {
	/// Layer identifier.
	pub layer: String,
	/// Stable item reference.
	pub item_ref: Value,
	/// Selection state such as selected, dropped, available, or reviewable.
	pub selection_state: String,
	/// Authority layer that owns the row.
	pub authority_layer: String,
	/// Freshness or temporal state.
	pub freshness_state: String,
	/// Source refs or source snapshots backing the row.
	pub source_refs: Value,
	/// Optional final score.
	pub score: Option<f32>,
	/// Optional rank within the layer.
	pub rank: Option<u32>,
	/// Short selection rationale.
	pub rationale: Option<String>,
	/// Stage reason for selected/dropped status.
	pub stage_reason: Option<String>,
	/// Replay command or deterministic artifact path when available.
	pub replay_command: Option<String>,
	/// Row-level evidence class.
	pub evidence_class: String,
	/// Layer-specific debug artifacts.
	pub debug_artifacts: Value,
}
