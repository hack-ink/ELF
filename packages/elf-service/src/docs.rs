//! Document ingestion and retrieval APIs.

mod api;
mod chunking;
mod excerpts;
mod queries;
mod search_support;
mod service;
mod source_capture;
mod types;
mod validation;

pub use api::{
	DocRetrievalTrajectory, DocRetrievalTrajectoryStage, DocType, DocsDeleteRequest,
	DocsDeleteResponse, DocsExcerptLocator, DocsExcerptResponse, DocsExcerptVerification,
	DocsExcerptsGetRequest, DocsGetRequest, DocsGetResponse, DocsPutRequest, DocsPutResponse,
	DocsSearchL0Item, DocsSearchL0ItemHashes, DocsSearchL0ItemLocator, DocsSearchL0ItemPointer,
	DocsSearchL0ItemReference, DocsSearchL0ItemState, DocsSearchL0Request, DocsSearchL0Response,
	DocsSourceCaptureSummary, DocsSourceSpanRef, TextPositionSelector, TextQuoteSelector,
};

use std::{
	collections::{HashMap, HashSet},
	slice,
};

use qdrant_client::{
	Qdrant,
	qdrant::{
		Condition, DatetimeRange, Document, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query,
		QueryPointsBuilder, ScoredPoint, Timestamp, point_id::PointIdOptions,
	},
};
use serde_json::{Map, Value};
use sqlx::{PgExecutor, PgPool};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokenizers::Tokenizer;
use uuid::Uuid;

use crate::{
	ElfService, Error, NoteOp, Result,
	access::{ORG_PROJECT_ID, SharedSpaceGrantKey},
};
use chunking::{load_tokenizer, split_tokens_by_offsets};
use elf_config::Config;
use elf_domain::{
	english_gate,
	writegate::{self, WritePolicyAudit},
};
use elf_storage::{
	doc_outbox, docs,
	models::{DocChunk, DocDocument},
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};
#[cfg(test)] use excerpts::should_enable_sparse_auto;
use excerpts::{
	build_doc_search_filter, build_docs_l0_pointer, doc_read_allowed, docs_excerpt_locator,
	docs_excerpts_resolve_windowed_match, docs_search_sparse_enabled, load_docs_excerpt_context,
	parse_scored_point_uuid_id, truncate_bytes,
};
use queries::{load_doc_search_rows, run_doc_fusion_query};
use search_support::{
	apply_doc_recency_boost, docs_search_l0_deduplicated_chunks, docs_search_l0_project_items,
	record_result_projection_stage,
};
use source_capture::{
	build_doc_chunk_rows, build_source_capture_summary, doc_chunk_id_for,
	normalize_source_ref_for_capture, source_record_id_for, source_span_id,
};
use types::{
	ByteChunk, DEFAULT_DOC_MAX_BYTES, DEFAULT_L0_MAX_BYTES, DEFAULT_L1_MAX_BYTES,
	DEFAULT_L2_MAX_BYTES, DEFAULT_MAX_CHUNKS_PER_DOC, DOC_SOURCE_CAPTURE_SCHEMA_V1,
	DOC_SOURCE_REF_RESOLVER_V1, DOC_SOURCE_REF_SCHEMA_V1, DOC_SOURCE_SPAN_SCHEMA_V1, DOC_STATUSES,
	DocChunkingProfile, DocExcerptMatch, DocExcerptRange, DocSearchRow, DocTrajectoryBuilder,
	DocsSearchL0Filters, DocsSearchL0FiltersParsed, DocsSearchL0Prepared, DocsSearchL0RangesParsed,
	DocsSparseMode, ExcerptsSelectorKind, MAX_CANDIDATE_K, MAX_TOP_K, SOURCE_LIBRARY_FIELD_KEYS,
	SOURCE_LIBRARY_KINDS, SOURCE_LIBRARY_TRUST_LABELS, SourceCaptureSummaryInput, ValidatedDocsPut,
};
use validation::{
	excerpt_level_max, resolve_doc_chunking_profile, validate_docs_excerpts_get, validate_docs_put,
	validate_docs_search_l0,
};
#[cfg(test)]
#[path = "docs/tests.rs"]
mod tests;
