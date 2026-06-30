//! Cross-layer recall/debug panel readback.

mod helpers;
mod layers;
mod replay;
mod sources;
mod trace;
mod types;

pub use types::{
	ELF_RECALL_DEBUG_PANEL_SCHEMA_V1, ELF_RECALL_TRACE_SCHEMA_V1, RecallDebugLayer,
	RecallDebugPanelRequest, RecallDebugPanelRequestEcho, RecallDebugPanelResponse,
	RecallDebugPanelSummary, RecallDebugRow, RecallTrace, RecallTraceEntry, RecallTraceSummary,
};

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	DocsSearchL0Request, DreamingReviewQueueRequest, ElfService, Error, GraphQueryPredicateRef,
	GraphReportRequest, KnowledgePageSearchItem, KnowledgePageSearchRequest, Result,
	SearchExplainItem, SearchTrace, SearchTrajectoryStage, TraceBundleGetRequest,
	access::{ORG_PROJECT_ID, SharedSpaceGrantKey},
	search::{TraceBundleMode, TraceReplayCandidate},
};
use elf_storage::models::MemoryNote;
use helpers::{
	candidate_identity, candidate_is_selected, freshness_from_note_source, graph_replay_command,
	graph_temporal_status, json_anchor, knowledge_freshness, last_stage_name, public_error_class,
	search_item_candidate_key, source_ref_from_note_source,
};
use replay::{candidate_debug_row, memory_compact_replay_artifact};
use sources::{note_debug_read_allowed, note_debug_source_pair};
use trace::{
	blocked_layer, build_recall_trace, layer_from_rows, layer_from_rows_with_artifacts,
	not_requested_layer, summarize_layers,
};
use types::{
	constants::{DEFAULT_RECALL_DEBUG_LIMIT, MAX_RECALL_DEBUG_DOCS_LIMIT, MAX_RECALL_DEBUG_LIMIT},
	source_record::NoteDebugSourceRow,
};
#[cfg(test)]
#[path = "recall_debug/tests.rs"]
mod tests;
