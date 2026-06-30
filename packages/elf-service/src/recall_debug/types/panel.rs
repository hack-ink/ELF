use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

use crate::recall_debug::{RecallDebugPanelRequestEcho, RecallDebugRow, RecallTrace};

/// Cross-layer recall/debug panel response.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugPanelResponse {
	/// Response schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Panel generation timestamp.
	pub generated_at: OffsetDateTime,
	/// Echo of the effective anchors used for this response.
	pub request: RecallDebugPanelRequestEcho,
	/// Aggregate panel summary.
	pub summary: RecallDebugPanelSummary,
	/// Deterministic flat trace projection for agents and fixture assertions.
	pub recall_trace: RecallTrace,
	/// Cross-layer rows grouped by source layer.
	pub layers: Vec<RecallDebugLayer>,
}

/// Aggregate panel counters.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RecallDebugPanelSummary {
	/// Number of returned layers.
	pub layer_count: usize,
	/// Total returned row count.
	pub row_count: usize,
	/// Rows selected by a retrieval or review stage.
	pub selected_count: usize,
	/// Rows dropped by a retrieval or review stage.
	pub dropped_count: usize,
	/// Rows available for inspection but not selected/dropped.
	pub available_count: usize,
	/// Layers skipped because no anchor was supplied.
	pub not_requested_layer_count: usize,
	/// Layers that require follow-up before they can prove a debug claim.
	pub incomplete_layer_count: usize,
	/// Rows or layers that require raw SQL to inspect.
	pub raw_sql_needed_count: usize,
	/// Rows with a replay command or deterministic artifact path.
	pub replay_command_count: usize,
	/// Evidence-class counts across layers.
	pub evidence_class_counts: BTreeMap<String, usize>,
}

/// One recall/debug source layer.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugLayer {
	/// Layer identifier.
	pub layer: String,
	/// Evidence class for this layer.
	pub evidence_class: String,
	/// Human-readable layer summary.
	pub summary: String,
	/// Query or object anchor used by the layer.
	pub anchor: Option<String>,
	/// Number of returned rows.
	pub row_count: usize,
	/// Selected rows in this layer.
	pub selected_count: usize,
	/// Dropped rows in this layer.
	pub dropped_count: usize,
	/// Available review/inspection rows in this layer.
	pub available_count: usize,
	/// Whether raw SQL is needed to inspect this layer.
	pub raw_sql_needed: bool,
	/// Whether the layer includes replay commands or deterministic artifact paths.
	pub replayable: bool,
	/// Compact layer-level debug artifacts.
	pub debug_artifacts: Value,
	/// Returned layer rows.
	pub rows: Vec<RecallDebugRow>,
}
