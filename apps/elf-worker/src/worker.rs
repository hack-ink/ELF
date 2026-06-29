//! Worker runtime and queue-processing helpers.

mod consolidation_jobs;
mod doc_indexing;
mod helpers;
mod note_indexing;
mod outbox_jobs;
mod runtime;
mod trace_jobs;
mod types;

pub use self::{
	runtime::{process_once, run_worker},
	types::WorkerState,
};

use std::{collections::HashMap, slice, string::ToString};

use qdrant_client::{
	Payload, QdrantError,
	qdrant::{
		Condition, DeletePointsBuilder, Document, Filter, PointStruct, UpsertPointsBuilder, Vector,
	},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgConnection, PgExecutor, QueryBuilder};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{Error, Result};
use consolidation_jobs::handle_consolidation_job;
use doc_indexing::{handle_doc_delete, handle_doc_upsert};
use elf_chunking::{Chunk, ChunkingConfig, Tokenizer};
use elf_config::EmbeddingProviderConfig;
use elf_domain::consolidation::{
	CONSOLIDATION_CONTRACT_SCHEMA_V1, ConsolidationJobPayload, ConsolidationProposalContract,
	ConsolidationReviewState, ConsolidationRunState, ConsolidationValidationError,
};
use elf_providers::embedding;
use elf_storage::{
	consolidation::{self, ConsolidationRunStateUpdate},
	db::Db,
	doc_outbox, docs,
	models::{
		ConsolidationProposal, ConsolidationRunJob, DocIndexingOutboxEntry, IndexingOutboxEntry,
		MemoryNote, TraceOutboxJob,
	},
	outbox,
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME, QdrantStore},
	queries,
};
use helpers::{
	backoff_for_attempt, build_chunk_records, encode_json, format_timestamp, format_vector_text,
	is_not_found_error, mean_pool, note_is_active, project_doc_ref_fields, sanitize_outbox_error,
	to_std_duration, validate_vector_dim,
};
use note_indexing::{handle_delete, handle_upsert};
use outbox_jobs::{
	process_consolidation_run_job_once, process_doc_indexing_outbox_once,
	process_indexing_outbox_once, process_trace_outbox_once,
};
use trace_jobs::{
	handle_trace_job, purge_expired_cache, purge_expired_search_sessions,
	purge_expired_trace_candidates, purge_expired_traces,
};
use types::{
	BASE_BACKOFF_MS, CLAIM_LEASE_SECONDS, CONSOLIDATION_JOB_LEASE_SECONDS, ChunkRecord,
	DocChunkIndexRow, MAX_BACKOFF_MS, MAX_OUTBOX_ERROR_CHARS, NoteFieldRow, POLL_INTERVAL_MS,
	ProjectDocRefFields, TRACE_CLEANUP_INTERVAL_SECONDS, TRACE_OUTBOX_LEASE_SECONDS,
	TraceCandidateInsert, TraceCandidateRecord, TraceItemInsert, TraceItemRecord, TracePayload,
	TraceRecord, TraceStageInsert, TraceStageItemInsert, TraceTrajectoryStageRecord,
};

#[cfg(test)]
#[path = "worker/tests.rs"]
mod tests;
