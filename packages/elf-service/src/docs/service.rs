mod excerpt_get;
mod l0_search;
mod put;
mod read;

use crate::{
	access,
	docs::{
		DocDocument, DocExcerptRange, DocSearchRow, DocTrajectoryBuilder, DocsDeleteRequest,
		DocsDeleteResponse, DocsExcerptResponse, DocsExcerptVerification, DocsExcerptsGetRequest,
		DocsGetRequest, DocsGetResponse, DocsPutRequest, DocsPutResponse, DocsSearchL0Filters,
		DocsSearchL0Item, DocsSearchL0Prepared, DocsSearchL0Request, DocsSearchL0Response,
		DocsSparseMode, ElfService, Error, HashMap, HashSet, MAX_CANDIDATE_K, MAX_TOP_K, NoteOp,
		ORG_PROJECT_ID, OffsetDateTime, Result, ScoredPoint, SharedSpaceGrantKey,
		SourceCaptureSummaryInput, Uuid, ValidatedDocsPut, apply_doc_recency_boost,
		build_doc_chunk_rows, build_doc_search_filter, build_source_capture_summary,
		doc_chunk_id_for, doc_outbox, doc_read_allowed, docs, docs_excerpt_locator,
		docs_excerpts_resolve_windowed_match, docs_search_l0_deduplicated_chunks,
		docs_search_l0_project_items, docs_search_sparse_enabled, excerpt_level_max,
		load_doc_search_rows, load_docs_excerpt_context, load_tokenizer,
		normalize_source_ref_for_capture, record_result_projection_stage,
		resolve_doc_chunking_profile, run_doc_fusion_query, slice, source_record_id_for,
		split_tokens_by_offsets, validate_docs_excerpts_get, validate_docs_put,
		validate_docs_search_l0,
	},
	search,
};
