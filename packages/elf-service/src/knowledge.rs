//! Deterministic derived knowledge page rebuild and readback service APIs.

mod api;
mod lint;
mod persistence;
mod read;
mod rebuild;
mod resolve;
mod responses;
mod sections;
mod sources;
mod support;
mod types;
mod watch;
mod watch_service;

pub use api::{
	KnowledgeDeltaMemoryCandidate, KnowledgePageChangedSource, KnowledgePageGetRequest,
	KnowledgePageLintFindingResponse, KnowledgePageLintRequest, KnowledgePageLintResponse,
	KnowledgePageLintSummary, KnowledgePageProposalRunSummary, KnowledgePageRebuildOutput,
	KnowledgePageRebuildRequest, KnowledgePageRebuildResponse, KnowledgePageResponse,
	KnowledgePageSearchItem, KnowledgePageSearchRequest, KnowledgePageSearchResponse,
	KnowledgePageSectionRebuildState, KnowledgePageSectionResponse,
	KnowledgePageSectionSourceBacklink, KnowledgePageSourceRefResponse, KnowledgePageSummary,
	KnowledgePageWatchRebuildItem, KnowledgePageWatchRebuildRequest,
	KnowledgePageWatchRebuildResponse, KnowledgePageWatchRebuildSummary, KnowledgePagesListRequest,
	KnowledgePagesListResponse,
};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::{self, Map, Number, Value};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result, access,
	consolidation::{
		ConsolidationProposalInput, ConsolidationRunCreateRequest, ConsolidationRunCreateResponse,
	},
	search,
};
use elf_domain::{
	consolidation::{
		ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
		ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
		ConsolidationSourceKind, ConsolidationSourceSnapshot,
	},
	english_gate,
	knowledge::{
		KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1, KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
		KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1, KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
		KNOWLEDGE_PAGE_WATCH_REBUILD_SCHEMA_V1, KnowledgePageKind, KnowledgeSourceKind,
	},
};
use elf_storage::{
	knowledge::{
		self, KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeEventSource,
		KnowledgeNoteSource, KnowledgePageLintFindingInsert, KnowledgePageSearchRow,
		KnowledgePageSectionInsert, KnowledgePageSourceRefInsert, KnowledgePageUpsert,
		KnowledgeProposalSource, KnowledgeRelationSource, KnowledgeRelationSourcesFetch,
	},
	models::{KnowledgePage, KnowledgePageSection, KnowledgePageSourceRef},
};
use persistence::{insert_lint_finding, replace_page_children};
use responses::{knowledge_page_search_item, section_response};
use sections::{build_sections, lint_page_sections, lint_unsupported_sections};
use sources::{
	cloned_source_refs, recallable_source_refs, source_refs_by_section, source_row_read_allowed,
	source_snapshots,
};
#[cfg(test)] use support::hash_text;
use support::{
	bounded_limit, citation_count, citations_value, coverage_complete, current_key,
	doc_chunk_source_snapshot, doc_source_snapshot, empty_object, event_source_snapshot,
	generated_title, hash_json, low_source_coverage_finding, missing_source_finding,
	note_source_snapshot, page_content_hash, previous_version_diff_from_metadata,
	previous_version_diff_value, proposal_source_snapshot, rebuild_metadata,
	rebuild_metadata_with_previous_version_diff, relation_source_snapshot,
	repair_guidance_for_finding_type, sanitize_proposal_snapshot, section_hash_payload,
	snippet_for_query, sorted_unique, source_changed, source_coverage_value, source_indexes,
	source_key, source_snapshot_value, source_sort_key, stale_source_finding, truncate_chars,
	validate_context, validate_non_empty, validate_object, version_identity_value,
	with_repair_guidance,
};
use types::{DraftSection, LintDraft, SourceIds, SourceSnapshot, WatchRebuildOutcome};
use watch::{
	blocked_watch_rebuild, candidate_proposal_input, candidate_run_input_refs,
	changed_source_arrays, default_generate_memory_candidates, knowledge_delta_source_snapshot,
	normalized_changed_sources, proposal_run_summary, rebuild_request_from_page,
	successful_watch_rebuild, watch_operator_summary, watch_rebuild_summary,
};
#[cfg(test)] use watch::{memory_candidates_for_page, rebuild_outputs};

const DEFAULT_LIST_LIMIT: i64 = 50;
const MAX_LIST_LIMIT: i64 = 200;
const SEARCH_SNIPPET_CHARS: usize = 280;
const PREVIOUS_VERSION_DIFF_KEY: &str = "previous_version_diff";
#[cfg(test)]
#[path = "knowledge/tests.rs"]
mod tests;
