/// Schema identifier for recall/debug panel responses.
pub const ELF_RECALL_DEBUG_PANEL_SCHEMA_V1: &str = "elf.recall_debug_panel/v1";
/// Schema identifier for deterministic recall trace projections.
pub const ELF_RECALL_TRACE_SCHEMA_V1: &str = "elf.recall_trace/v1";

pub(in crate::recall_debug) const DEFAULT_RECALL_DEBUG_LIMIT: u32 = 25;
pub(in crate::recall_debug) const MAX_RECALL_DEBUG_LIMIT: u32 = 100;
pub(in crate::recall_debug) const MAX_RECALL_DEBUG_DOCS_LIMIT: u32 = 32;
