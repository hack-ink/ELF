mod candidates;
mod inputs;
mod outcomes;
mod outputs;
mod states;
mod summary;

pub(super) use self::{
	candidates::{
		candidate_proposal_input, candidate_run_input_refs, knowledge_delta_source_snapshot,
		memory_candidates_for_page, proposal_run_summary,
	},
	inputs::{
		changed_source_arrays, default_generate_memory_candidates, normalized_changed_sources,
		rebuild_request_from_page,
	},
	outcomes::{blocked_watch_rebuild, successful_watch_rebuild},
	outputs::{blocked_outputs, candidate_reasons_by_section, rebuild_outputs},
	states::{blocked_section_states, successful_rebuild_state, successful_section_states},
	summary::{page_operator_summary, watch_operator_summary, watch_rebuild_summary},
};

use crate::knowledge::{
	BTreeMap, BTreeSet, ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage,
	ConsolidationMarker, ConsolidationMarkerSeverity, ConsolidationMarkers,
	ConsolidationProposalDiff, ConsolidationProposalInput, ConsolidationRunCreateResponse,
	ConsolidationSourceKind, ConsolidationSourceSnapshot, Error, KnowledgeDeltaMemoryCandidate,
	KnowledgePage, KnowledgePageChangedSource, KnowledgePageKind, KnowledgePageProposalRunSummary,
	KnowledgePageRebuildOutput, KnowledgePageRebuildRequest, KnowledgePageResponse,
	KnowledgePageSection, KnowledgePageSectionRebuildState, KnowledgePageSectionResponse,
	KnowledgePageSourceRef, KnowledgePageSourceRefResponse, KnowledgePageWatchRebuildItem,
	KnowledgePageWatchRebuildSummary, KnowledgeSourceKind, LintDraft, Result, SourceIds, Uuid,
	Value, WatchRebuildOutcome, empty_object, previous_version_diff_from_metadata, serde_json,
	truncate_chars,
};
