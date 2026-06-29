use crate::recall_debug::{
	BTreeMap, Deserialize, GraphQueryEntityRef, GraphQueryPredicateRef, OffsetDateTime, Serialize,
	Uuid, Value,
};

/// Schema identifier for recall/debug panel responses.
pub const ELF_RECALL_DEBUG_PANEL_SCHEMA_V1: &str = "elf.recall_debug_panel/v1";
/// Schema identifier for deterministic recall trace projections.
pub const ELF_RECALL_TRACE_SCHEMA_V1: &str = "elf.recall_trace/v1";

pub(super) const DEFAULT_RECALL_DEBUG_LIMIT: u32 = 25;
pub(super) const MAX_RECALL_DEBUG_LIMIT: u32 = 100;
pub(super) const MAX_RECALL_DEBUG_DOCS_LIMIT: u32 = 32;

/// Request payload for the cross-layer recall/debug panel.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecallDebugPanelRequest {
	/// Tenant that owns the readback.
	pub tenant_id: String,
	/// Project that owns the readback.
	pub project_id: String,
	/// Agent requesting the readback.
	pub agent_id: String,
	/// Read profile used for memory, document, and graph visibility.
	pub read_profile: String,
	/// Optional search trace anchor for memory selected/dropped rows.
	pub trace_id: Option<Uuid>,
	/// Shared query used when docs_query or knowledge_query are omitted.
	pub query: Option<String>,
	/// Optional Source Library query.
	pub docs_query: Option<String>,
	/// Optional Knowledge Workspace page query.
	pub knowledge_query: Option<String>,
	/// Optional graph subject selector.
	pub graph_subject: Option<GraphQueryEntityRef>,
	/// Optional graph predicate selector.
	pub graph_predicate: Option<GraphQueryPredicateRef>,
	/// Whether to include Dreaming review queue proposals. Omitted means not requested.
	pub include_dreaming: Option<bool>,
	/// Maximum rows per layer.
	pub limit: Option<u32>,
	#[serde(skip)]
	/// Whether project-scoped trace anchors are allowed for an admin mirror request.
	pub allow_project_trace_debug: bool,
}

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

/// Stable request echo for panel responses.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugPanelRequestEcho {
	/// Search trace anchor used for memory rows.
	pub trace_id: Option<Uuid>,
	/// Effective Source Library query.
	pub docs_query: Option<String>,
	/// Effective Knowledge Workspace query.
	pub knowledge_query: Option<String>,
	/// Whether a graph subject was supplied.
	pub graph_subject_supplied: bool,
	/// Whether Dreaming proposals were included.
	pub include_dreaming: bool,
	/// Effective row cap per layer.
	pub limit: u32,
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

#[derive(Clone, Debug)]
pub(super) struct NoteDebugSourceRow {
	pub(super) status: String,
	pub(super) source_ref: Value,
	pub(super) updated_at: OffsetDateTime,
}
