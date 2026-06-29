mod coverage;
mod diff;
mod findings;
mod hash;
mod keys;
mod metadata;
mod snapshots;
mod text;
mod validation;

pub(super) use self::{
	coverage::{
		citation_count, citations_value, coverage_complete, source_coverage_value, source_indexes,
		source_snapshot_value,
	},
	diff::previous_version_diff_value,
	findings::{
		low_source_coverage_finding, missing_source_finding, repair_guidance_for_finding_type,
		source_changed, stale_source_finding, with_repair_guidance,
	},
	hash::{hash_json, hash_json_lossy, hash_text},
	keys::{
		bounded_limit, current_key, sorted_unique, source_key, source_sort_key, source_span_id,
	},
	metadata::{
		page_content_hash, previous_version_diff_from_metadata, rebuild_metadata,
		rebuild_metadata_with_previous_version_diff, section_hash_payload, version_identity_value,
	},
	snapshots::{
		doc_chunk_source_snapshot, doc_source_snapshot, event_source_snapshot,
		note_source_snapshot, proposal_source_snapshot, relation_source_snapshot,
		sanitize_proposal_snapshot,
	},
	text::{generated_title, normalize_whitespace, note_prefix, snippet_for_query, truncate_chars},
	validation::{empty_object, validate_context, validate_non_empty, validate_object},
};

use crate::knowledge::{
	BTreeMap, BTreeSet, DEFAULT_LIST_LIMIT, DraftSection, Error, KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
	KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1, KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1,
	KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1, KnowledgeDocChunkSource, KnowledgeDocSource,
	KnowledgeEventSource, KnowledgeNoteSource, KnowledgePage, KnowledgePageKind,
	KnowledgePageRebuildRequest, KnowledgePageSection, KnowledgePageSourceRef,
	KnowledgeProposalSource, KnowledgeRelationSource, KnowledgeSourceKind, LintDraft,
	MAX_LIST_LIMIT, Map, Number, PREVIOUS_VERSION_DIFF_KEY, Result, SourceSnapshot, Uuid, Value,
	serde_json,
};
