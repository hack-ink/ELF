//! Document ingestion and retrieval APIs.

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
use sqlx::{FromRow, PgExecutor, PgPool};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokenizers::Tokenizer;
use uuid::Uuid;

use crate::{
	ElfService, Error, NoteOp, Result,
	access::{self, ORG_PROJECT_ID, SharedSpaceGrantKey},
	search,
};
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

mod types;
use types::*;
mod api;
pub use api::*;
mod source_capture;
use source_capture::*;
mod validation;
use validation::*;
mod chunking;
use chunking::*;
mod search_support;
use search_support::*;
mod excerpts;
use excerpts::*;
mod queries;
use queries::*;
mod service;

#[cfg(test)]
#[path = "docs/tests.rs"]
mod tests;
