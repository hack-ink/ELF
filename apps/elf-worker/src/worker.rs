//! Worker runtime and queue-processing helpers.

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

mod types;
pub use types::WorkerState;
use types::*;
mod helpers;
use helpers::*;
mod consolidation_jobs;
use consolidation_jobs::*;
mod note_indexing;
use note_indexing::*;
mod doc_indexing;
use doc_indexing::*;
mod trace_jobs;
use trace_jobs::*;
mod outbox_jobs;
use outbox_jobs::*;
mod runtime;
pub use runtime::{process_once, run_worker};

#[cfg(test)]
#[path = "worker/tests.rs"]
mod tests;
