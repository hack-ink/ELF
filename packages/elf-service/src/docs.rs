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
use serde::{Deserialize, Serialize};
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
	writegate::{self, WritePolicy, WritePolicyAudit},
};
use elf_storage::{
	doc_outbox, docs,
	models::{DocChunk, DocDocument},
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};

const MAX_TOP_K: u32 = 32;
const MAX_CANDIDATE_K: u32 = 1_024;
const DEFAULT_DOC_MAX_BYTES: usize = 4 * 1_024 * 1_024;
const DEFAULT_MAX_CHUNKS_PER_DOC: usize = 4_096;
const DEFAULT_L0_MAX_BYTES: usize = 256;
const DEFAULT_L1_MAX_BYTES: usize = 8 * 1_024;
const DEFAULT_L2_MAX_BYTES: usize = 32 * 1_024;
const DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1: &str = "doc_retrieval_trajectory/v1";
const DOC_SOURCE_REF_SCHEMA_V1: &str = "source_ref/v1";
const DOC_SOURCE_REF_RESOLVER_V1: &str = "elf_doc_ext/v1";
const DOC_SOURCE_CAPTURE_SCHEMA_V1: &str = "doc_source_capture/v1";
const DOC_SOURCE_SPAN_SCHEMA_V1: &str = "doc_source_span/v1";
const DOC_STATUSES: [&str; 2] = ["active", "deleted"];
const SOURCE_LIBRARY_FIELD_KEYS: [&str; 9] = [
	"source_kind",
	"canonical_uri",
	"captured_at",
	"source_created_at",
	"trust_label",
	"author",
	"handle",
	"excerpt_locator",
	"source_content_hash",
];
const SOURCE_LIBRARY_KINDS: [&str; 7] =
	["article", "social_thread", "pdf", "text_export", "repo_file", "chat_excerpt", "web_page"];
const SOURCE_LIBRARY_TRUST_LABELS: [&str; 5] =
	["trusted", "user_captured", "public_web", "third_party", "unverified"];

/// Document classification used for persistence and retrieval filters.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
	/// Long-lived knowledge-base material.
	Knowledge,
	/// Chat transcripts or conversational context.
	Chat,
	/// Search-produced reference material.
	Search,
	/// Development-oriented artifacts such as code or plans.
	Dev,
}
impl DocType {
	/// Returns the canonical storage and API string for this document type.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Knowledge => "knowledge",
			Self::Chat => "chat",
			Self::Search => "search",
			Self::Dev => "dev",
		}
	}

	/// Parses a canonical document-type string.
	pub fn parse(raw_doc_type: &str) -> Result<Self> {
		match raw_doc_type {
			"knowledge" => Ok(Self::Knowledge),
			"chat" => Ok(Self::Chat),
			"search" => Ok(Self::Search),
			"dev" => Ok(Self::Dev),
			_ => Err(Error::InvalidRequest {
				message: "doc_type must be one of: knowledge, chat, search, dev.".to_string(),
			}),
		}
	}
}

/// Request payload for document ingestion.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsPutRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent ingesting the document.
	pub agent_id: String,
	/// Scope to assign to the document.
	pub scope: String,
	/// Optional raw document-type string.
	pub doc_type: Option<String>,
	/// Optional display title for the document.
	pub title: Option<String>,
	/// Optional write policy applied before persistence.
	pub write_policy: Option<WritePolicy>,
	#[serde(default)]
	/// Structured provenance metadata for the document.
	pub source_ref: Value,
	/// Full document body to store and chunk.
	pub content: String,
}

/// Response payload for document ingestion.
#[derive(Clone, Debug, Serialize)]
pub struct DocsPutResponse {
	/// Identifier of the stored document.
	pub doc_id: Uuid,
	/// Normalized Source Library capture metadata for the stored document.
	pub source_capture: DocsSourceCaptureSummary,
	/// Number of persisted chunks generated from the content.
	pub chunk_count: u32,
	/// Byte length of the stored content.
	pub content_bytes: u32,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Write-policy audit emitted for the stored document, when applicable.
	pub write_policy_audit: Option<WritePolicyAudit>,
}

/// Normalized Source Library capture metadata returned by `docs_put`.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSourceCaptureSummary {
	/// Schema identifier for this capture summary.
	pub schema: String,
	/// Stable source record identifier. This is also the stored `doc_id`.
	pub source_record_id: Uuid,
	/// Canonical source origin used for operator inspection and deduplication.
	pub origin: String,
	/// RFC3339 timestamp when ELF captured the source.
	pub captured_at: String,
	/// Whole-document BLAKE3 hash for the persisted content.
	pub content_hash: String,
	/// Visibility scope assigned to the source record.
	pub visibility_scope: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional display title associated with the source record.
	pub title: Option<String>,
	/// Normalized source type, derived from `source_kind` when present.
	pub source_type: String,
	/// Stable span references for persisted source chunks.
	pub source_spans: Vec<DocsSourceSpanRef>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	/// Typed audit records for redacted or excluded source spans.
	pub policy_spans: Vec<DocsSourceSpanRef>,
}

/// Stable reference to one captured or policy-affected source span.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSourceSpanRef {
	/// Schema identifier for this span reference.
	pub schema: String,
	/// Stable span identifier derived from content hash and byte offsets.
	pub span_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Chunk identifier when this span is backed by a persisted chunk.
	pub chunk_id: Option<Uuid>,
	/// Span lifecycle status such as `captured`, `excluded`, or `redacted`.
	pub status: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Typed reason code for non-captured spans.
	pub reason_code: Option<String>,
	/// Inclusive start byte offset in the relevant content hash.
	pub start_offset: usize,
	/// Exclusive end byte offset in the relevant content hash.
	pub end_offset: usize,
	/// Whole-content hash that makes the offsets replayable.
	pub content_hash: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Chunk hash when this span is backed by a persisted chunk.
	pub chunk_hash: Option<String>,
}

/// Request payload for document metadata lookup.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsGetRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Identifier of the document to fetch.
	pub doc_id: Uuid,
}

/// Response payload for document metadata lookup.
#[derive(Clone, Debug, Serialize)]
pub struct DocsGetResponse {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent that ingested the document.
	pub agent_id: String,
	/// Scope key for the document.
	pub scope: String,
	/// Stored document type.
	pub doc_type: String,
	/// Lifecycle status for the document.
	pub status: String,
	/// Optional document title.
	pub title: Option<String>,
	/// Structured provenance metadata.
	pub source_ref: Value,
	/// Byte length of the stored content.
	pub content_bytes: u32,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Request payload for Source Library document deletion.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsDeleteRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent requesting the deletion.
	pub agent_id: String,
	/// Identifier of the document to delete.
	pub doc_id: Uuid,
}

/// Response payload for Source Library document deletion.
#[derive(Clone, Debug, Serialize)]
pub struct DocsDeleteResponse {
	/// Identifier of the affected document.
	pub doc_id: Uuid,
	/// Operation that was applied.
	pub op: NoteOp,
	/// Number of persisted chunks queued for derived-index deletion.
	pub chunk_delete_count: u32,
}

/// Request payload for L0 document retrieval.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsSearchL0Request {
	/// Tenant to search within.
	pub tenant_id: String,
	/// Project to search within.
	pub project_id: String,
	/// Agent used for access-control checks.
	pub caller_agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Search query text.
	pub query: String,
	/// Optional scope filter.
	pub scope: Option<String>,
	/// Optional status filter.
	pub status: Option<String>,
	/// Optional document-type filter.
	pub doc_type: Option<String>,
	/// Sparse-retrieval mode override.
	pub sparse_mode: Option<String>,
	/// Optional domain filter from source metadata.
	pub domain: Option<String>,
	/// Optional repository filter from source metadata.
	pub repo: Option<String>,
	/// Optional agent filter.
	pub agent_id: Option<String>,
	/// Optional thread filter.
	pub thread_id: Option<String>,
	/// Optional lower bound for `updated_at`.
	pub updated_after: Option<String>,
	/// Optional upper bound for `updated_at`.
	pub updated_before: Option<String>,
	/// Optional lower bound for source timestamp metadata.
	pub ts_gte: Option<String>,
	/// Optional upper bound for source timestamp metadata.
	pub ts_lte: Option<String>,
	/// Maximum number of returned items.
	pub top_k: Option<u32>,
	/// Retrieval breadth before deduplication and projection.
	pub candidate_k: Option<u32>,
	/// When true, includes retrieval trajectory output.
	pub explain: Option<bool>,
}

/// One chunk-level hit returned by `docs_search_l0`.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Item {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Stable pointer bundle for later excerpt or resolution workflows.
	pub pointer: DocsSearchL0ItemPointer,
	/// Final score after retrieval and boosting.
	pub score: f32,
	/// Returned snippet text.
	pub snippet: String,
	/// Scope key for the document.
	pub scope: String,
	/// Stored document type.
	pub doc_type: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent that ingested the document.
	pub agent_id: String,
	/// Last update timestamp for the document.
	pub updated_at: OffsetDateTime,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Chunk-level BLAKE3 hash.
	pub chunk_hash: String,
}

/// Response payload for `docs_search_l0`.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Response {
	/// Retrieval trace identifier.
	pub trace_id: Uuid,
	/// Returned chunk hits.
	pub items: Vec<DocsSearchL0Item>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional retrieval trajectory emitted in explain mode.
	pub trajectory: Option<DocRetrievalTrajectory>,
}

/// Stable pointer for a chunk hit returned by document search.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemPointer {
	/// Pointer schema identifier.
	pub schema: String,
	/// Pointer resolver identifier.
	pub resolver: String,
	#[serde(rename = "ref")]
	/// Logical identifiers used by the resolver.
	pub reference: DocsSearchL0ItemReference,
	/// Freshness guard for the pointer target.
	pub state: DocsSearchL0ItemState,
	/// Hash aliases for simpler pointer consumers.
	pub hashes: DocsSearchL0ItemHashes,
	/// Selector hints that can hydrate this chunk through `docs_excerpts_get`.
	pub locator: DocsSearchL0ItemLocator,
}

/// Logical identifiers for a document-search hit.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemReference {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Stable source record identifier.
	pub source_record_id: Uuid,
	/// Stable source span identifier for this chunk.
	pub source_span_id: Uuid,
}

/// Freshness guard for a document-search hit.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemState {
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Chunk-level BLAKE3 hash.
	pub chunk_hash: String,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp for the document.
	pub doc_updated_at: OffsetDateTime,
}

/// Hash values carried with a document-search pointer.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemHashes {
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Chunk-level BLAKE3 hash.
	pub chunk_hash: String,
}

/// Locator hints carried with a document-search pointer.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemLocator {
	/// Stable source span identifier for the locator.
	pub span_id: Uuid,
	/// Chunk byte position in the authoritative document content.
	pub position: TextPositionSelector,
}

/// Explain payload for a document retrieval run.
#[derive(Clone, Debug, Serialize)]
pub struct DocRetrievalTrajectory {
	/// Trajectory schema identifier.
	pub schema: String,
	/// Ordered retrieval stages.
	pub stages: Vec<DocRetrievalTrajectoryStage>,
}

/// One stage in a document retrieval trajectory.
#[derive(Clone, Debug, Serialize)]
pub struct DocRetrievalTrajectoryStage {
	/// Zero-based stage order.
	pub stage_order: u32,
	/// Stable stage name.
	pub stage_name: String,
	/// Free-form stage statistics.
	pub stats: Value,
}

/// Quote-based selector for excerpt extraction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextQuoteSelector {
	/// Exact quote text to resolve.
	pub exact: String,
	/// Optional leading context used to disambiguate repeated quotes.
	pub prefix: Option<String>,
	/// Optional trailing context used to disambiguate repeated quotes.
	pub suffix: Option<String>,
}

/// Byte-position selector for excerpt extraction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextPositionSelector {
	/// Inclusive start byte offset.
	pub start: usize,
	/// Exclusive end byte offset.
	pub end: usize,
}

/// Request payload for excerpt retrieval.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsExcerptsGetRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Identifier of the source document.
	pub doc_id: Uuid,
	/// Excerpt budget level: `L0`, `L1`, or `L2`.
	pub level: String, // "L0" | "L1" | "L2"
	/// Optional chunk identifier when the caller already knows the chunk.
	pub chunk_id: Option<Uuid>,
	/// Optional quote-based selector.
	pub quote: Option<TextQuoteSelector>,
	/// Optional byte-position selector.
	pub position: Option<TextPositionSelector>,
	/// When true, includes retrieval trajectory output.
	pub explain: Option<bool>,
}

/// Verification metadata for one extracted excerpt.
#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptVerification {
	/// Whether the excerpt selectors verified against current content.
	pub verified: bool,
	/// Verification failure codes.
	pub verification_errors: Vec<String>,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// BLAKE3 hash of the returned excerpt.
	pub excerpt_hash: String,
}

/// Response payload for excerpt retrieval.
#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptResponse {
	/// Excerpt trace identifier.
	pub trace_id: Uuid,
	/// Identifier of the source document.
	pub doc_id: Uuid,
	/// Returned excerpt text.
	pub excerpt: String,
	/// Inclusive start offset of the returned window.
	pub start_offset: usize,
	/// Exclusive end offset of the returned window.
	pub end_offset: usize,
	/// Concrete selector resolution result.
	pub locator: DocsExcerptLocator,
	/// Verification metadata for the returned excerpt.
	pub verification: DocsExcerptVerification,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional retrieval trajectory emitted in explain mode.
	pub trajectory: Option<DocRetrievalTrajectory>,
}

/// Selector resolution metadata for an excerpt.
#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptLocator {
	/// Stable source span identifier for the matched selector span.
	pub span_id: Uuid,
	/// Selector kind that produced the match.
	pub selector_kind: String,
	/// Inclusive start offset of the matched selector span.
	pub match_start_offset: usize,
	/// Exclusive end offset of the matched selector span.
	pub match_end_offset: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Matched chunk identifier, when known.
	pub chunk_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Quote selector actually used for resolution.
	pub quote: Option<TextQuoteSelector>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Position selector actually used for resolution.
	pub position: Option<TextPositionSelector>,
}

struct SourceCaptureSummaryInput<'a> {
	doc_id: Uuid,
	source_ref: &'a Map<String, Value>,
	doc_type: DocType,
	scope: &'a str,
	title: Option<&'a str>,
	content_hash: &'a str,
	raw_content_hash: &'a str,
	now: OffsetDateTime,
	chunks: &'a [DocChunk],
	write_policy_audit: Option<&'a WritePolicyAudit>,
}

#[derive(Clone, Copy)]
struct DocExcerptMatch {
	selector_kind: ExcerptsSelectorKind,
	match_start_offset: usize,
	match_end_offset: usize,
}

struct DocExcerptRange {
	selector_kind: ExcerptsSelectorKind,
	match_start_offset: usize,
	match_end_offset: usize,
	start_offset: usize,
	end_offset: usize,
}

struct DocTrajectoryBuilder {
	explain: bool,
	stages: Vec<DocRetrievalTrajectoryStage>,
	stage_order: u32,
}
impl DocTrajectoryBuilder {
	fn new(explain: bool) -> Self {
		Self { explain, stages: Vec::new(), stage_order: 0 }
	}

	fn push(&mut self, stage_name: &str, stats: Value) {
		if !self.explain {
			return;
		}

		self.stages.push(DocRetrievalTrajectoryStage {
			stage_order: self.stage_order,
			stage_name: stage_name.to_string(),
			stats,
		});

		self.stage_order += 1;
	}

	fn into_trajectory(self) -> Option<DocRetrievalTrajectory> {
		if !self.explain {
			return None;
		}

		Some(DocRetrievalTrajectory {
			schema: DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1.to_string(),
			stages: self.stages,
		})
	}
}

#[derive(Clone, Debug)]
struct DocsSearchL0Filters {
	scope: Option<String>,
	status: String,
	doc_type: Option<DocType>,
	sparse_mode: DocsSparseMode,
	domain: Option<String>,
	repo: Option<String>,
	agent_id: Option<String>,
	thread_id: Option<String>,
	updated_after: Option<OffsetDateTime>,
	updated_before: Option<OffsetDateTime>,
	ts_gte: Option<OffsetDateTime>,
	ts_lte: Option<OffsetDateTime>,
}

#[derive(Clone, Copy, Debug)]
struct DocChunkingProfile {
	max_tokens: usize,
	overlap_tokens: usize,
	max_chunks: usize,
}

#[derive(Clone, Debug)]
struct ByteChunk {
	chunk_id: Uuid,
	start_offset: usize,
	end_offset: usize,
	text: String,
}

#[derive(Debug)]
struct ValidatedDocsPut {
	doc_type: DocType,
	content: String,
	write_policy_audit: Option<WritePolicyAudit>,
}

#[derive(Clone, Debug, FromRow)]
struct DocSearchRow {
	chunk_id: Uuid,
	doc_id: Uuid,
	scope: String,
	doc_type: String,
	project_id: String,
	agent_id: String,
	updated_at: OffsetDateTime,
	content_hash: String,
	chunk_hash: String,
	start_offset: i32,
	end_offset: i32,
	chunk_text: String,
}

struct DocsSearchL0Prepared {
	top_k: u32,
	candidate_k: u32,
	sparse_mode: DocsSparseMode,
	sparse_enabled: bool,
	now: OffsetDateTime,
	trajectory: DocTrajectoryBuilder,
	allowed_scopes: Vec<String>,
	shared_grants: HashSet<SharedSpaceGrantKey>,
	filter: Filter,
	vector: Vec<f32>,
	status: String,
}

#[derive(Debug)]
struct DocsSearchL0FiltersParsed {
	scope: Option<String>,
	status: String,
	doc_type: Option<DocType>,
	sparse_mode: DocsSparseMode,
	domain: Option<String>,
	repo: Option<String>,
	agent_id: Option<String>,
	thread_id: Option<String>,
}

#[derive(Debug)]
struct DocsSearchL0RangesParsed {
	updated_after: Option<OffsetDateTime>,
	updated_before: Option<OffsetDateTime>,
	ts_gte: Option<OffsetDateTime>,
	ts_lte: Option<OffsetDateTime>,
}

impl ElfService {
	/// Validates, chunks, stores, and enqueues a document for indexing.
	pub async fn docs_put(&self, req: DocsPutRequest) -> Result<DocsPutResponse> {
		let ValidatedDocsPut { doc_type, content, write_policy_audit } = validate_docs_put(&req)?;
		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let chunking_profile = resolve_doc_chunking_profile(doc_type);
		let tokenizer = load_tokenizer(&self.cfg)?;
		let tenant_id = req.tenant_id.clone();
		let project_id = req.project_id.clone();
		let agent_id = req.agent_id.clone();
		let scope = req.scope.clone();
		let title = req.title.clone();
		let source_ref = req.source_ref.clone();
		let source_ref_map = source_ref.as_object().ok_or_else(|| Error::InvalidRequest {
			message: "source_ref must be a JSON object.".to_string(),
		})?;
		let effective_project_id =
			if scope.trim() == "org_shared" { ORG_PROJECT_ID } else { project_id.as_str() };
		let content_bytes = content.len();
		let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
		let raw_content_hash = blake3::hash(req.content.as_bytes()).to_hex().to_string();
		let doc_id = source_record_id_for(
			tenant_id.as_str(),
			effective_project_id,
			agent_id.as_str(),
			scope.as_str(),
			doc_type,
			source_ref_map,
			content_hash.as_str(),
		);
		let mut chunks = split_tokens_by_offsets(
			content.as_str(),
			chunking_profile.max_tokens,
			chunking_profile.overlap_tokens,
			chunking_profile.max_chunks,
			&tokenizer,
		)?;

		for (chunk_index, chunk) in chunks.iter_mut().enumerate() {
			chunk.chunk_id = doc_chunk_id_for(doc_id, chunk_index as i32);
		}

		let chunk_rows = build_doc_chunk_rows(doc_id, &chunks, now);
		let source_capture = build_source_capture_summary(SourceCaptureSummaryInput {
			doc_id,
			source_ref: source_ref_map,
			doc_type,
			scope: scope.as_str(),
			title: title.as_deref(),
			content_hash: content_hash.as_str(),
			raw_content_hash: raw_content_hash.as_str(),
			now,
			chunks: &chunk_rows,
			write_policy_audit: write_policy_audit.as_ref(),
		})?;
		let normalized_source_ref = normalize_source_ref_for_capture(source_ref, &source_capture)?;
		let doc_row = DocDocument {
			doc_id,
			tenant_id: tenant_id.clone(),
			project_id: effective_project_id.to_string(),
			agent_id: agent_id.clone(),
			scope: scope.clone(),
			doc_type: doc_type.as_str().to_string(),
			status: "active".to_string(),
			title,
			source_ref: docs::normalize_source_ref(Some(normalized_source_ref)),
			content,
			content_bytes: content_bytes as i32,
			content_hash: content_hash.clone(),
			created_at: now,
			updated_at: now,
		};
		let mut tx = self.db.pool.begin().await?;

		docs::insert_doc_document(&mut *tx, &doc_row).await?;

		for chunk_row in &chunk_rows {
			docs::insert_doc_chunk(&mut *tx, chunk_row).await?;
			doc_outbox::enqueue_doc_outbox(
				&mut *tx,
				doc_id,
				chunk_row.chunk_id,
				"UPSERT",
				embed_version.as_str(),
			)
			.await?;
		}

		if scope.trim() != "agent_private" {
			access::ensure_active_project_scope_grant(
				&mut *tx,
				tenant_id.as_str(),
				effective_project_id,
				scope.as_str(),
				agent_id.as_str(),
			)
			.await?;
		}

		tx.commit().await?;

		Ok(DocsPutResponse {
			doc_id,
			source_capture,
			chunk_count: chunk_rows.len() as u32,
			content_bytes: content_bytes as u32,
			content_hash,
			write_policy_audit,
		})
	}

	/// Loads document metadata when the caller can read the requested scope.
	pub async fn docs_get(&self, req: DocsGetRequest) -> Result<DocsGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		let read_profile = req.read_profile.trim();

		if tenant_id.is_empty()
			|| project_id.is_empty()
			|| agent_id.is_empty()
			|| read_profile.is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, agent_id, and read_profile are required."
					.to_string(),
			});
		}

		let allowed_scopes = search::resolve_read_profile_scopes(&self.cfg, read_profile)?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let row: Option<DocDocument> = sqlx::query_as::<_, DocDocument>(
			"\
SELECT
	doc_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	doc_type,
	status,
	title,
	COALESCE(source_ref, '{}'::jsonb) AS source_ref,
	content,
	content_bytes,
	content_hash,
	created_at,
	updated_at
FROM doc_documents
WHERE doc_id = $1
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )
LIMIT 1",
		)
		.bind(req.doc_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ORG_PROJECT_ID)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		};
		let shared_grants = if row.scope == "agent_private" {
			HashSet::new()
		} else {
			access::load_shared_read_grants_with_org_shared(
				&self.db.pool,
				tenant_id,
				project_id,
				agent_id,
				org_shared_allowed,
			)
			.await?
		};

		if row.status != "active"
			|| !doc_read_allowed(
				agent_id,
				&allowed_scopes,
				&shared_grants,
				row.agent_id.as_str(),
				row.scope.as_str(),
			) {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		}

		Ok(DocsGetResponse {
			doc_id: row.doc_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			scope: row.scope,
			doc_type: row.doc_type,
			status: row.status,
			title: row.title,
			source_ref: row.source_ref,
			content_bytes: row.content_bytes.max(0) as u32,
			content_hash: row.content_hash,
			created_at: row.created_at,
			updated_at: row.updated_at,
		})
	}

	/// Soft-deletes one Source Library document and enqueues doc-vector deletion.
	pub async fn docs_delete(&self, req: DocsDeleteRequest) -> Result<DocsDeleteResponse> {
		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let mut tx = self.db.pool.begin().await?;
		let row: DocDocument = sqlx::query_as::<_, DocDocument>(
			"\
SELECT
	doc_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	doc_type,
	status,
	title,
	COALESCE(source_ref, '{}'::jsonb) AS source_ref,
	content,
	content_bytes,
	content_hash,
	created_at,
	updated_at
FROM doc_documents
WHERE doc_id = $1
	AND tenant_id = $2
	AND (
		project_id = $3
		OR (project_id = $4 AND scope = 'org_shared')
	)
FOR UPDATE",
		)
		.bind(req.doc_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ORG_PROJECT_ID)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| Error::NotFound { message: "Doc not found.".to_string() })?;

		if row.agent_id != agent_id {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		}

		let scope_allowed = self.cfg.scopes.allowed.iter().any(|scope| scope == &row.scope);
		let write_allowed = match row.scope.as_str() {
			"agent_private" => self.cfg.scopes.write_allowed.agent_private,
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};

		if !scope_allowed || !write_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}
		if row.status == "deleted" {
			tx.commit().await?;

			return Ok(DocsDeleteResponse {
				doc_id: row.doc_id,
				op: NoteOp::None,
				chunk_delete_count: 0,
			});
		}

		let chunks = docs::list_doc_chunks(&mut *tx, row.doc_id).await?;

		docs::mark_doc_deleted(&mut *tx, tenant_id, row.doc_id, now).await?;

		for chunk in &chunks {
			doc_outbox::enqueue_doc_outbox(
				&mut *tx,
				row.doc_id,
				chunk.chunk_id,
				"DELETE",
				embed_version.as_str(),
			)
			.await?;
		}

		tx.commit().await?;

		Ok(DocsDeleteResponse {
			doc_id: row.doc_id,
			op: NoteOp::Delete,
			chunk_delete_count: chunks.len() as u32,
		})
	}

	/// Runs L0 document retrieval with access filtering and optional explain output.
	pub async fn docs_search_l0(&self, req: DocsSearchL0Request) -> Result<DocsSearchL0Response> {
		let trace_id = Uuid::new_v4();
		let filters = validate_docs_search_l0(&req)?;
		let mut prepared = self.prepare_docs_search_l0_request(&req, &filters).await?;
		let scored = run_doc_fusion_query(
			&self.qdrant.client,
			self.cfg.storage.qdrant.docs_collection.as_str(),
			req.query.as_str(),
			&prepared.vector,
			&prepared.filter,
			prepared.sparse_mode,
			prepared.candidate_k,
		)
		.await?;

		self.record_docs_search_l0_vector_stats(
			&mut prepared.trajectory,
			&scored,
			prepared.sparse_enabled,
			prepared.sparse_mode,
		);

		let scored_chunks =
			docs_search_l0_deduplicated_chunks(&scored, prepared.candidate_k as usize)?;
		let chunk_ids: Vec<Uuid> = scored_chunks.iter().map(|(chunk_id, _)| *chunk_id).collect();
		let rows = self
			.load_doc_search_rows(&req, &prepared.status, &chunk_ids, &mut prepared.trajectory)
			.await?;
		let mut items = self.build_docs_search_l0_items(
			&req,
			&scored_chunks,
			&rows,
			&prepared.allowed_scopes,
			&prepared.shared_grants,
			&mut prepared.trajectory,
		);

		apply_doc_recency_boost(
			&mut items,
			prepared.now,
			self.cfg.ranking.recency_tau_days,
			self.cfg.ranking.tie_breaker_weight,
		);

		items.sort_by(|a, b| b.score.total_cmp(&a.score));
		items.truncate(prepared.top_k as usize);

		record_result_projection_stage(
			&mut prepared.trajectory,
			rows.len(),
			items.len(),
			self.cfg.ranking.recency_tau_days,
			self.cfg.ranking.tie_breaker_weight,
		);

		Ok(DocsSearchL0Response {
			trace_id,
			items,
			trajectory: prepared.trajectory.into_trajectory(),
		})
	}

	async fn load_doc_search_rows(
		&self,
		req: &DocsSearchL0Request,
		status: &str,
		chunk_ids: &[Uuid],
		trajectory: &mut DocTrajectoryBuilder,
	) -> Result<HashMap<Uuid, DocSearchRow>> {
		let rows = load_doc_search_rows(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			status,
			chunk_ids,
		)
		.await?;

		trajectory.push(
			"chunk_lookup",
			serde_json::json!({
				"requested_chunks": chunk_ids.len(),
				"loaded_chunks": rows.len(),
			}),
		);

		Ok(rows)
	}

	fn build_docs_search_l0_items(
		&self,
		req: &DocsSearchL0Request,
		scored_chunks: &[(Uuid, f32)],
		rows: &HashMap<Uuid, DocSearchRow>,
		allowed_scopes: &[String],
		shared_grants: &HashSet<SharedSpaceGrantKey>,
		trajectory: &mut DocTrajectoryBuilder,
	) -> Vec<DocsSearchL0Item> {
		let items = docs_search_l0_project_items(
			scored_chunks,
			rows,
			req.caller_agent_id.as_str(),
			allowed_scopes,
			shared_grants,
		);

		trajectory.push(
			"dedupe",
			serde_json::json!({
				"raw_candidates": scored_chunks.len(),
				"deduped_candidates": items.len(),
			}),
		);

		items
	}

	async fn prepare_docs_search_l0_request(
		&self,
		req: &DocsSearchL0Request,
		filters: &DocsSearchL0Filters,
	) -> Result<DocsSearchL0Prepared> {
		let explain = req.explain.unwrap_or(false);
		let top_k = req.top_k.unwrap_or(12).min(MAX_TOP_K);
		let candidate_k = req.candidate_k.unwrap_or(60).min(MAX_CANDIDATE_K);
		let sparse_mode = filters.sparse_mode;
		let sparse_enabled = docs_search_sparse_enabled(sparse_mode, req.query.as_str());
		let now = OffsetDateTime::now_utc();
		let mut trajectory = DocTrajectoryBuilder::new(explain);

		trajectory.push(
			"request_validation",
			serde_json::json!({
				"query_len": req.query.len(),
				"top_k": top_k,
				"candidate_k": candidate_k,
				"sparse_mode": sparse_mode.as_str(),
				"doc_type": filters
					.doc_type
					.as_ref()
				.map(|doc_type| doc_type.as_str())
				.unwrap_or("<default>"),
				"status": &filters.status,
			}),
		);

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.as_str())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.caller_agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let filter = build_doc_search_filter(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.caller_agent_id.as_str(),
			&allowed_scopes,
			filters,
		);
		let embedded = self
			.providers
			.embedding
			.embed(&self.cfg.providers.embedding, slice::from_ref(&req.query))
			.await?;

		trajectory.push("query_embedding", serde_json::json!({ "provider": "embedding" }));

		let vector = embedded.first().ok_or_else(|| Error::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		})?;

		trajectory.push(
			"vector_dimension_check",
			serde_json::json!({
				"provided_dim": vector.len(),
				"expected_dim": self.cfg.storage.qdrant.vector_dim as usize,
			}),
		);

		if vector.len() != self.cfg.storage.qdrant.vector_dim as usize {
			return Err(Error::Provider {
				message: "Embedding vector dimension mismatch.".to_string(),
			});
		}

		Ok(DocsSearchL0Prepared {
			top_k,
			candidate_k,
			sparse_mode,
			sparse_enabled,
			now,
			trajectory,
			allowed_scopes,
			shared_grants,
			filter,
			vector: vector.to_vec(),
			status: filters.status.clone(),
		})
	}

	fn record_docs_search_l0_vector_stats(
		&self,
		trajectory: &mut DocTrajectoryBuilder,
		scored: &[ScoredPoint],
		sparse_enabled: bool,
		sparse_mode: DocsSparseMode,
	) {
		let channels = if sparse_enabled { vec!["dense", "sparse"] } else { vec!["dense"] };

		trajectory.push(
			"vector_search",
			serde_json::json!({
				"raw_points": scored.len(),
				"sparse_mode": sparse_mode.as_str(),
				"channels": channels,
			}),
		);
	}

	/// Resolves and verifies an excerpt window from quote, position, or chunk selectors.
	pub async fn docs_excerpts_get(
		&self,
		req: DocsExcerptsGetRequest,
	) -> Result<DocsExcerptResponse> {
		let explain = req.explain.unwrap_or(false);
		let trace_id = Uuid::new_v4();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		let read_profile = req.read_profile.trim();
		let mut trajectory = DocTrajectoryBuilder::new(explain);

		trajectory.push(
			"request_validation",
			serde_json::json!({
				"doc_id": req.doc_id,
				"read_profile": read_profile,
			}),
		);

		validate_docs_excerpts_get(
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			req.quote.as_ref(),
		)?;

		let doc = load_docs_excerpt_context(
			&self.cfg,
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			req.doc_id,
		)
		.await?;
		let level_max = excerpt_level_max(req.level.as_str())?;

		trajectory.push(
			"level_selection",
			serde_json::json!({
				"level": req.level,
				"max_bytes": level_max,
			}),
		);

		let mut verified = true;
		let mut verification_errors = Vec::new();
		let DocExcerptRange {
			selector_kind,
			match_start_offset,
			match_end_offset,
			start_offset,
			end_offset,
		} = docs_excerpts_resolve_windowed_match(
			&self.db.pool,
			&doc,
			&req,
			level_max,
			&mut trajectory,
			&mut verified,
			&mut verification_errors,
		)
		.await?;
		let excerpt = doc.content.get(start_offset..end_offset).unwrap_or("").to_string();

		if excerpt.is_empty() {
			verified = false;

			verification_errors.push("EMPTY_EXCERPT".to_string());
		}

		let excerpt_hash = blake3::hash(excerpt.as_bytes()).to_hex().to_string();

		trajectory.push(
			"verification",
			serde_json::json!({
				"verified": verified,
				"error_count": verification_errors.len(),
			}),
		);

		Ok(DocsExcerptResponse {
			trace_id,
			doc_id: doc.doc_id,
			excerpt,
			start_offset,
			end_offset,
			locator: docs_excerpt_locator(
				&req,
				&selector_kind,
				match_start_offset,
				match_end_offset,
				doc.content_hash.as_str(),
			),
			verification: DocsExcerptVerification {
				verified,
				verification_errors,
				content_hash: doc.content_hash.clone(),
				excerpt_hash,
			},
			trajectory: trajectory.into_trajectory(),
		})
	}
}

#[derive(Clone, Copy, Debug)]
enum DocsSparseMode {
	Auto,
	On,
	Off,
}
impl DocsSparseMode {
	fn as_str(self) -> &'static str {
		match self {
			Self::Auto => "auto",
			Self::On => "on",
			Self::Off => "off",
		}
	}
}

#[derive(Clone, Copy)]
enum ExcerptsSelectorKind {
	ChunkId,
	Quote,
	Position,
}
impl ExcerptsSelectorKind {
	fn as_str(&self) -> &'static str {
		match self {
			Self::ChunkId => "chunk_id",
			Self::Quote => "quote",
			Self::Position => "position",
		}
	}

	fn span_kind(&self) -> &'static str {
		match self {
			Self::ChunkId => "captured",
			Self::Quote => "quote",
			Self::Position => "position",
		}
	}
}

fn docs_search_l0_deduplicated_chunks(
	scored: &[ScoredPoint],
	candidate_k: usize,
) -> Result<Vec<(Uuid, f32)>> {
	let mut seen = HashSet::new();
	let mut chunks = Vec::new();

	for point in scored.iter().take(candidate_k) {
		let chunk_id = parse_scored_point_uuid_id(point)?;

		if seen.insert(chunk_id) {
			chunks.push((chunk_id, point.score));
		}
	}

	Ok(chunks)
}

fn docs_search_l0_project_items(
	scored_chunks: &[(Uuid, f32)],
	rows: &HashMap<Uuid, DocSearchRow>,
	caller_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
) -> Vec<DocsSearchL0Item> {
	let mut items = Vec::with_capacity(scored_chunks.len());

	for (chunk_id, score) in scored_chunks {
		let Some(row) = rows.get(chunk_id) else { continue };

		if !doc_read_allowed(
			caller_agent_id,
			allowed_scopes,
			shared_grants,
			row.agent_id.as_str(),
			row.scope.as_str(),
		) {
			continue;
		}

		items.push(DocsSearchL0Item {
			doc_id: row.doc_id,
			chunk_id: *chunk_id,
			pointer: build_docs_l0_pointer(row, *chunk_id),
			score: *score,
			snippet: truncate_bytes(row.chunk_text.as_str(), DEFAULT_L0_MAX_BYTES),
			scope: row.scope.clone(),
			doc_type: row.doc_type.clone(),
			project_id: row.project_id.clone(),
			agent_id: row.agent_id.clone(),
			updated_at: row.updated_at,
			content_hash: row.content_hash.clone(),
			chunk_hash: row.chunk_hash.clone(),
		});
	}

	items
}

fn apply_doc_recency_boost(
	items: &mut [DocsSearchL0Item],
	now: OffsetDateTime,
	recency_tau_days: f32,
	tie_breaker_weight: f32,
) {
	if tie_breaker_weight <= 0.0 || items.is_empty() {
		return;
	}

	for item in items.iter_mut() {
		let age_days = ((now - item.updated_at).as_seconds_f32() / 86_400.0).max(0.0);
		let recency_decay =
			if recency_tau_days > 0.0 { (-age_days / recency_tau_days).exp() } else { 1.0 };

		item.score += tie_breaker_weight * recency_decay;
	}
}

fn record_result_projection_stage(
	trajectory: &mut DocTrajectoryBuilder,
	pre_authorization_candidates: usize,
	returned_items: usize,
	recency_tau_days: f32,
	tie_breaker_weight: f32,
) {
	trajectory.push(
		"result_projection",
		serde_json::json!({
			"pre_authorization_candidates": pre_authorization_candidates,
			"returned_items": returned_items,
			"recency_tau_days": recency_tau_days,
			"tie_breaker_weight": tie_breaker_weight,
			"recency_boost_applied": tie_breaker_weight > 0.0 && !pre_authorization_candidates.eq(&0),
		}),
	)
}

fn docs_excerpt_locator(
	req: &DocsExcerptsGetRequest,
	selector_kind: &ExcerptsSelectorKind,
	match_start_offset: usize,
	match_end_offset: usize,
	content_hash: &str,
) -> DocsExcerptLocator {
	DocsExcerptLocator {
		span_id: source_span_id(
			content_hash,
			match_start_offset,
			match_end_offset,
			selector_kind.span_kind(),
		),
		selector_kind: selector_kind.as_str().to_string(),
		match_start_offset,
		match_end_offset,
		chunk_id: req.chunk_id,
		quote: req.quote.clone(),
		position: req.position.clone(),
	}
}

fn build_docs_l0_pointer(row: &DocSearchRow, chunk_id: Uuid) -> DocsSearchL0ItemPointer {
	let hashes = DocsSearchL0ItemHashes {
		content_hash: row.content_hash.clone(),
		chunk_hash: row.chunk_hash.clone(),
	};

	DocsSearchL0ItemPointer {
		schema: DOC_SOURCE_REF_SCHEMA_V1.to_string(),
		resolver: DOC_SOURCE_REF_RESOLVER_V1.to_string(),
		reference: DocsSearchL0ItemReference {
			doc_id: row.doc_id,
			chunk_id,
			source_record_id: row.doc_id,
			source_span_id: source_span_id(
				row.content_hash.as_str(),
				row.start_offset.max(0) as usize,
				row.end_offset.max(0) as usize,
				"captured",
			),
		},
		state: DocsSearchL0ItemState {
			content_hash: hashes.content_hash.clone(),
			chunk_hash: hashes.chunk_hash.clone(),
			doc_updated_at: row.updated_at,
		},
		hashes,
		locator: DocsSearchL0ItemLocator {
			span_id: source_span_id(
				row.content_hash.as_str(),
				row.start_offset.max(0) as usize,
				row.end_offset.max(0) as usize,
				"captured",
			),
			position: TextPositionSelector {
				start: row.start_offset.max(0) as usize,
				end: row.end_offset.max(0) as usize,
			},
		},
	}
}

fn build_doc_chunk_rows(doc_id: Uuid, chunks: &[ByteChunk], now: OffsetDateTime) -> Vec<DocChunk> {
	chunks
		.iter()
		.enumerate()
		.map(|(chunk_index, chunk)| DocChunk {
			chunk_id: doc_chunk_id_for(doc_id, chunk_index as i32),
			doc_id,
			chunk_index: chunk_index as i32,
			start_offset: chunk.start_offset as i32,
			end_offset: chunk.end_offset as i32,
			chunk_text: chunk.text.clone(),
			chunk_hash: blake3::hash(chunk.text.as_bytes()).to_hex().to_string(),
			created_at: now,
		})
		.collect()
}

fn doc_chunk_id_for(doc_id: Uuid, chunk_index: i32) -> Uuid {
	let name = format!("elf-doc-chunk/v1:{doc_id}:{chunk_index}");

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

fn source_record_id_for(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	doc_type: DocType,
	source_ref: &Map<String, Value>,
	content_hash: &str,
) -> Uuid {
	let name = serde_json::json!([
		"elf-doc-source-record/v1",
		tenant_id,
		project_id,
		agent_id,
		scope,
		doc_type.as_str(),
		source_identity_value(source_ref, doc_type),
		content_hash,
	])
	.to_string();

	Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes())
}

fn source_span_id(content_hash: &str, start: usize, end: usize, span_kind: &str) -> Uuid {
	let name = serde_json::json!(["elf-doc-source-span/v1", content_hash, start, end, span_kind,])
		.to_string();

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

fn build_source_capture_summary(
	input: SourceCaptureSummaryInput<'_>,
) -> Result<DocsSourceCaptureSummary> {
	let SourceCaptureSummaryInput {
		doc_id,
		source_ref,
		doc_type,
		scope,
		title,
		content_hash,
		raw_content_hash,
		now,
		chunks,
		write_policy_audit,
	} = input;
	let captured_at = source_ref
		.get("captured_at")
		.and_then(Value::as_str)
		.map(ToString::to_string)
		.unwrap_or(format_timestamp(now)?);
	let source_spans = chunks
		.iter()
		.map(|chunk| DocsSourceSpanRef {
			schema: DOC_SOURCE_SPAN_SCHEMA_V1.to_string(),
			span_id: source_span_id(
				content_hash,
				chunk.start_offset.max(0) as usize,
				chunk.end_offset.max(0) as usize,
				"captured",
			),
			chunk_id: Some(chunk.chunk_id),
			status: "captured".to_string(),
			reason_code: None,
			start_offset: chunk.start_offset.max(0) as usize,
			end_offset: chunk.end_offset.max(0) as usize,
			content_hash: content_hash.to_string(),
			chunk_hash: Some(chunk.chunk_hash.clone()),
		})
		.collect();
	let policy_spans = source_policy_spans(raw_content_hash, write_policy_audit);

	Ok(DocsSourceCaptureSummary {
		schema: DOC_SOURCE_CAPTURE_SCHEMA_V1.to_string(),
		source_record_id: doc_id,
		origin: source_origin(source_ref, doc_type),
		captured_at,
		content_hash: content_hash.to_string(),
		visibility_scope: scope.to_string(),
		title: title.map(ToString::to_string),
		source_type: source_type(source_ref, doc_type),
		source_spans,
		policy_spans,
	})
}

fn source_policy_spans(
	raw_content_hash: &str,
	audit: Option<&WritePolicyAudit>,
) -> Vec<DocsSourceSpanRef> {
	let Some(audit) = audit else {
		return Vec::new();
	};
	let mut spans = Vec::with_capacity(audit.exclusions.len() + audit.redactions.len());

	for span in &audit.exclusions {
		spans.push(policy_span_ref(
			raw_content_hash,
			span.start,
			span.end,
			"excluded",
			"WRITE_POLICY_EXCLUSION",
		));
	}
	for redaction in &audit.redactions {
		spans.push(policy_span_ref(
			raw_content_hash,
			redaction.span.start,
			redaction.span.end,
			"redacted",
			"WRITE_POLICY_REDACTION",
		));
	}

	spans
}

fn policy_span_ref(
	content_hash: &str,
	start: usize,
	end: usize,
	status: &str,
	reason_code: &str,
) -> DocsSourceSpanRef {
	DocsSourceSpanRef {
		schema: DOC_SOURCE_SPAN_SCHEMA_V1.to_string(),
		span_id: source_span_id(content_hash, start, end, reason_code),
		chunk_id: None,
		status: status.to_string(),
		reason_code: Some(reason_code.to_string()),
		start_offset: start,
		end_offset: end,
		content_hash: content_hash.to_string(),
		chunk_hash: None,
	}
}

fn normalize_source_ref_for_capture(
	source_ref: Value,
	source_capture: &DocsSourceCaptureSummary,
) -> Result<Value> {
	let mut source_ref = source_ref.as_object().cloned().ok_or_else(|| Error::InvalidRequest {
		message: "source_ref must be a JSON object.".to_string(),
	})?;

	source_ref.insert(
		"source_record_id".to_string(),
		Value::String(source_capture.source_record_id.to_string()),
	);
	source_ref.insert("origin".to_string(), Value::String(source_capture.origin.clone()));
	source_ref.insert("captured_at".to_string(), Value::String(source_capture.captured_at.clone()));
	source_ref
		.insert("content_hash".to_string(), Value::String(source_capture.content_hash.clone()));
	source_ref.insert(
		"visibility_scope".to_string(),
		Value::String(source_capture.visibility_scope.clone()),
	);

	if let Some(title) = source_capture.title.as_ref() {
		source_ref.entry("title".to_string()).or_insert_with(|| Value::String(title.clone()));
	}

	source_ref.insert("source_type".to_string(), Value::String(source_capture.source_type.clone()));
	source_ref
		.insert("source_spans".to_string(), source_spans_to_value(&source_capture.source_spans)?);

	if !source_capture.policy_spans.is_empty() {
		source_ref.insert(
			"policy_spans".to_string(),
			source_spans_to_value(&source_capture.policy_spans)?,
		);
	}

	Ok(Value::Object(source_ref))
}

fn source_spans_to_value(spans: &[DocsSourceSpanRef]) -> Result<Value> {
	serde_json::to_value(spans).map_err(|err| Error::InvalidRequest {
		message: format!("failed to encode source span metadata: {err}"),
	})
}

fn source_type(source_ref: &Map<String, Value>, doc_type: DocType) -> String {
	source_ref
		.get("source_kind")
		.and_then(Value::as_str)
		.filter(|value| !value.trim().is_empty())
		.unwrap_or_else(|| doc_type.as_str())
		.to_string()
}

fn source_origin(source_ref: &Map<String, Value>, doc_type: DocType) -> String {
	if let Some(origin) = source_ref_string(source_ref, "canonical_uri")
		.or_else(|| source_ref_string(source_ref, "url"))
		.or_else(|| source_ref_string(source_ref, "uri"))
	{
		return origin.to_string();
	}

	match doc_type {
		DocType::Chat => source_ref_string(source_ref, "message_id")
			.map(|message_id| {
				format!(
					"thread:{}#{}",
					source_ref_string(source_ref, "thread_id").unwrap_or("unknown"),
					message_id
				)
			})
			.unwrap_or_else(|| {
				format!(
					"thread:{}",
					source_ref_string(source_ref, "thread_id").unwrap_or("unknown")
				)
			}),
		DocType::Search => source_ref_string(source_ref, "domain")
			.map(|domain| format!("search:{domain}"))
			.unwrap_or_else(|| "search:unknown".to_string()),
		DocType::Dev => dev_origin(source_ref),
		DocType::Knowledge => source_ref_string(source_ref, "ts")
			.map(|ts| format!("knowledge:{ts}"))
			.unwrap_or_else(|| "knowledge:unknown".to_string()),
	}
}

fn dev_origin(source_ref: &Map<String, Value>) -> String {
	let repo = source_ref_string(source_ref, "repo").unwrap_or("unknown");
	let path = source_ref_string(source_ref, "path").unwrap_or("");
	let revision = source_ref_string(source_ref, "commit_sha")
		.map(|commit| format!("@{commit}"))
		.or_else(|| source_ref_i64(source_ref, "pr_number").map(|pr| format!("#pr-{pr}")))
		.or_else(|| {
			source_ref_i64(source_ref, "issue_number").map(|issue| format!("#issue-{issue}"))
		})
		.unwrap_or_default();

	if path.is_empty() {
		format!("repo:{repo}{revision}")
	} else {
		format!("repo:{repo}/{path}{revision}")
	}
}

fn source_identity_value(source_ref: &Map<String, Value>, doc_type: DocType) -> Value {
	if let Some(canonical_uri) = source_ref_string(source_ref, "canonical_uri") {
		return serde_json::json!(["canonical_uri", canonical_uri]);
	}

	match doc_type {
		DocType::Chat => serde_json::json!([
			"chat",
			source_ref_string(source_ref, "thread_id"),
			source_ref_string(source_ref, "message_id"),
			source_ref_string(source_ref, "role"),
			source_ref_string(source_ref, "ts"),
		]),
		DocType::Search => serde_json::json!([
			"search",
			source_ref_string(source_ref, "url"),
			source_ref_string(source_ref, "domain"),
			source_ref_string(source_ref, "query"),
			source_ref_string(source_ref, "ts"),
		]),
		DocType::Dev => serde_json::json!([
			"dev",
			source_ref_string(source_ref, "repo"),
			source_ref_string(source_ref, "path"),
			source_ref_string(source_ref, "commit_sha"),
			source_ref_i64(source_ref, "pr_number"),
			source_ref_i64(source_ref, "issue_number"),
		]),
		DocType::Knowledge => serde_json::json!([
			"knowledge",
			source_ref_string(source_ref, "uri"),
			source_ref_string(source_ref, "ts"),
		]),
	}
}

fn source_ref_string<'a>(source_ref: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
	source_ref.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn source_ref_i64(source_ref: &Map<String, Value>, key: &str) -> Option<i64> {
	source_ref.get(key).and_then(Value::as_i64)
}

fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	ts.format(&Rfc3339).map_err(|err| Error::InvalidRequest {
		message: format!("failed to format RFC3339 timestamp: {err}"),
	})
}

fn resolve_doc_chunking_profile(doc_type: DocType) -> DocChunkingProfile {
	match doc_type {
		DocType::Chat | DocType::Search => DocChunkingProfile {
			max_tokens: 1_024,
			overlap_tokens: 128,
			max_chunks: DEFAULT_MAX_CHUNKS_PER_DOC,
		},
		DocType::Knowledge | DocType::Dev => DocChunkingProfile {
			max_tokens: 2_048,
			overlap_tokens: 256,
			max_chunks: DEFAULT_MAX_CHUNKS_PER_DOC,
		},
	}
}

fn validate_docs_excerpts_get(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
	quote: Option<&TextQuoteSelector>,
) -> Result<()> {
	if tenant_id.is_empty()
		|| project_id.is_empty()
		|| agent_id.is_empty()
		|| read_profile.is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, agent_id, and read_profile are required.".to_string(),
		});
	}

	if let Some(quote) = quote {
		validate_quote_selector_english(quote)?;
	}

	Ok(())
}

fn validate_quote_selector_english(quote: &TextQuoteSelector) -> Result<()> {
	if !english_gate::is_english_natural_language(quote.exact.as_str()) {
		return Err(Error::NonEnglishInput { field: "$.quote.exact".to_string() });
	}

	if let Some(prefix) = quote.prefix.as_ref()
		&& !english_gate::is_english_natural_language(prefix.as_str())
	{
		return Err(Error::NonEnglishInput { field: "$.quote.prefix".to_string() });
	}
	if let Some(suffix) = quote.suffix.as_ref()
		&& !english_gate::is_english_natural_language(suffix.as_str())
	{
		return Err(Error::NonEnglishInput { field: "$.quote.suffix".to_string() });
	}

	Ok(())
}

fn excerpt_level_max(level: &str) -> Result<usize> {
	match level {
		"L0" => Ok(DEFAULT_L0_MAX_BYTES),
		"L1" => Ok(DEFAULT_L1_MAX_BYTES),
		"L2" => Ok(DEFAULT_L2_MAX_BYTES),
		_ => Err(Error::InvalidRequest { message: "level must be L0, L1, or L2.".to_string() }),
	}
}

fn validate_docs_put(req: &DocsPutRequest) -> Result<ValidatedDocsPut> {
	if req.content.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "content must be non-empty.".to_string() });
	}
	if req.scope.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "scope must be non-empty.".to_string() });
	}
	if !matches!(req.scope.as_str(), "agent_private" | "project_shared" | "org_shared") {
		return Err(Error::InvalidRequest { message: "Unknown scope.".to_string() });
	}

	let source_ref = req.source_ref.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "source_ref must be a JSON object.".to_string(),
	})?;
	let source_ref_doc_type =
		extract_source_ref_string(source_ref, "doc_type", "$.source_ref[\"doc_type\"]")?;
	let source_ref_doc_type = DocType::parse(&source_ref_doc_type)?;
	let source_ref_schema =
		extract_source_ref_string(source_ref, "schema", "$.source_ref[\"schema\"]")?;

	if source_ref_schema != "doc_source_ref/v1" {
		return Err(Error::InvalidRequest {
			message: "source_ref.schema must be 'doc_source_ref/v1'.".to_string(),
		});
	}

	let ts = extract_source_ref_string(source_ref, "ts", "$.source_ref[\"ts\"]")?;

	OffsetDateTime::parse(ts.as_str(), &Rfc3339).map_err(|_| Error::InvalidRequest {
		message: "$.source_ref[\"ts\"] must be an RFC3339 datetime string.".to_string(),
	})?;

	let doc_type = if let Some(doc_type) = req.doc_type.as_ref() {
		let doc_type = DocType::parse(doc_type.as_str())?;

		if doc_type != source_ref_doc_type {
			return Err(Error::InvalidRequest {
				message: "doc_type must match source_ref.doc_type.".to_string(),
			});
		}

		doc_type
	} else {
		source_ref_doc_type
	};

	validate_doc_source_ref_requirements(source_ref_doc_type.as_str(), source_ref)?;
	validate_source_library_metadata(source_ref_doc_type.as_str(), source_ref)?;

	let write_policy =
		writegate::apply_write_policy(req.content.as_str(), req.write_policy.as_ref()).map_err(
			|err| Error::InvalidRequest { message: format!("write_policy is invalid: {err:?}") },
		)?;
	let write_policy_audit =
		if req.write_policy.is_some() { Some(write_policy.audit) } else { None };
	let content = write_policy.transformed;

	if content.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "content must be non-empty.".to_string() });
	}
	if content.len() > DEFAULT_DOC_MAX_BYTES {
		return Err(Error::InvalidRequest {
			message: "content exceeds max_doc_bytes.".to_string(),
		});
	}
	if writegate::contains_secrets(content.as_str()) {
		return Err(Error::InvalidRequest { message: "content contains secrets.".to_string() });
	}

	if let Some(found) = find_non_english_path(&req.source_ref, "$.source_ref") {
		return Err(Error::NonEnglishInput { field: found });
	}

	if !english_gate::is_english_natural_language(content.as_str()) {
		return Err(Error::NonEnglishInput { field: "$.content".to_string() });
	}

	if let Some(title) = req.title.as_ref()
		&& !english_gate::is_english_natural_language(title.as_str())
	{
		return Err(Error::NonEnglishInput { field: "$.title".to_string() });
	}

	Ok(ValidatedDocsPut { doc_type, content, write_policy_audit })
}

fn extract_source_ref_string(
	source_ref: &Map<String, Value>,
	key: &str,
	path: &str,
) -> Result<String> {
	source_ref
		.get(key)
		.and_then(Value::as_str)
		.map(|text| text.trim().to_string())
		.filter(|text| !text.is_empty())
		.ok_or_else(|| Error::InvalidRequest { message: format!("{path} is required.") })
}

fn validate_doc_source_ref_requirements(
	source_doc_type: &str,
	source_ref: &Map<String, Value>,
) -> Result<()> {
	match source_doc_type {
		"chat" => {
			extract_source_ref_string(source_ref, "thread_id", "$.source_ref[\"thread_id\"]")?;
			extract_source_ref_string(source_ref, "role", "$.source_ref[\"role\"]")?;
		},
		"search" => {
			extract_source_ref_string(source_ref, "query", "$.source_ref[\"query\"]")?;
			extract_source_ref_string(source_ref, "url", "$.source_ref[\"url\"]")?;
			extract_source_ref_string(source_ref, "domain", "$.source_ref[\"domain\"]")?;
		},
		"dev" => {
			extract_source_ref_string(source_ref, "repo", "$.source_ref[\"repo\"]")?;

			let commit_sha_present = source_ref
				.get("commit_sha")
				.and_then(Value::as_str)
				.is_some_and(|value| !value.trim().is_empty());
			let pr_number_present = source_ref
				.get("pr_number")
				.is_some_and(|value| value.as_i64().is_some() || value.as_u64().is_some());
			let issue_number_present = source_ref
				.get("issue_number")
				.is_some_and(|value| value.as_i64().is_some() || value.as_u64().is_some());
			let present_count =
				commit_sha_present as u8 + pr_number_present as u8 + issue_number_present as u8;

			if present_count != 1 {
				return Err(Error::InvalidRequest {
					message:
						"For doc_type=dev, exactly one of commit_sha, pr_number, or issue_number is required."
							.to_string(),
				});
			}
		},
		"knowledge" => {},
		_ => unreachable!(),
	}

	Ok(())
}

fn validate_source_library_metadata(
	source_doc_type: &str,
	source_ref: &Map<String, Value>,
) -> Result<()> {
	if !source_library_metadata_present(source_ref) {
		return Ok(());
	}

	let source_kind =
		extract_source_ref_string(source_ref, "source_kind", "$.source_ref[\"source_kind\"]")?;

	if !SOURCE_LIBRARY_KINDS.contains(&source_kind.as_str()) {
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"source_kind\"] must be one of: {}.",
				SOURCE_LIBRARY_KINDS.join("|")
			),
		});
	}

	validate_source_kind_doc_type(source_kind.as_str(), source_doc_type)?;
	extract_source_ref_string(source_ref, "canonical_uri", "$.source_ref[\"canonical_uri\"]")?;
	validate_source_ref_rfc3339(source_ref, "captured_at")?;

	if source_ref.contains_key("source_created_at") {
		validate_source_ref_rfc3339(source_ref, "source_created_at")?;
	}

	let trust_label =
		extract_source_ref_string(source_ref, "trust_label", "$.source_ref[\"trust_label\"]")?;

	if !SOURCE_LIBRARY_TRUST_LABELS.contains(&trust_label.as_str()) {
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"trust_label\"] must be one of: {}.",
				SOURCE_LIBRARY_TRUST_LABELS.join("|")
			),
		});
	}

	validate_optional_source_ref_string(source_ref, "author")?;
	validate_optional_source_ref_string(source_ref, "handle")?;
	validate_optional_source_ref_string(source_ref, "source_content_hash")?;

	if let Some(locator) = source_ref.get("excerpt_locator") {
		validate_source_library_excerpt_locator(locator)?;
	}

	Ok(())
}

fn source_library_metadata_present(source_ref: &Map<String, Value>) -> bool {
	SOURCE_LIBRARY_FIELD_KEYS.iter().any(|key| source_ref.contains_key(*key))
}

fn validate_source_kind_doc_type(source_kind: &str, source_doc_type: &str) -> Result<()> {
	let expected_doc_type = match source_kind {
		"social_thread" | "chat_excerpt" => Some("chat"),
		"repo_file" => Some("dev"),
		_ => None,
	};

	if let Some(expected_doc_type) = expected_doc_type
		&& source_doc_type != expected_doc_type
	{
		return Err(Error::InvalidRequest {
			message: format!(
				"$.source_ref[\"source_kind\"]={source_kind} requires doc_type={expected_doc_type}."
			),
		});
	}

	Ok(())
}

fn validate_source_ref_rfc3339(source_ref: &Map<String, Value>, key: &str) -> Result<()> {
	let path = format!("$.source_ref[\"{key}\"]");
	let value = extract_source_ref_string(source_ref, key, path.as_str())?;

	OffsetDateTime::parse(value.as_str(), &Rfc3339).map_err(|_| Error::InvalidRequest {
		message: format!("{path} must be an RFC3339 datetime string."),
	})?;

	Ok(())
}

fn validate_optional_source_ref_string(source_ref: &Map<String, Value>, key: &str) -> Result<()> {
	let path = format!("$.source_ref[\"{key}\"]");

	validate_optional_source_ref_string_at(source_ref, key, path.as_str())
}

fn validate_optional_source_ref_string_at(
	source_ref: &Map<String, Value>,
	key: &str,
	path: &str,
) -> Result<()> {
	let Some(value) = source_ref.get(key) else {
		return Ok(());
	};

	value.as_str().map(str::trim).filter(|value| !value.is_empty()).ok_or_else(|| {
		Error::InvalidRequest { message: format!("{path} must be a non-empty string.") }
	})?;

	Ok(())
}

fn validate_source_library_excerpt_locator(locator: &Value) -> Result<()> {
	let locator = locator.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "$.source_ref[\"excerpt_locator\"] must be a JSON object.".to_string(),
	})?;
	let has_quote = locator.contains_key("quote");
	let has_position = locator.contains_key("position");

	if !has_quote && !has_position {
		return Err(Error::InvalidRequest {
			message: "$.source_ref[\"excerpt_locator\"] requires quote or position.".to_string(),
		});
	}

	if let Some(quote) = locator.get("quote") {
		validate_source_library_quote_locator(quote)?;
	}
	if let Some(position) = locator.get("position") {
		validate_source_library_position_locator(position)?;
	}

	Ok(())
}

fn validate_source_library_quote_locator(quote: &Value) -> Result<()> {
	let quote = quote.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "$.source_ref[\"excerpt_locator\"][\"quote\"] must be a JSON object.".to_string(),
	})?;

	extract_source_ref_string(
		quote,
		"exact",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"exact\"]",
	)?;
	validate_optional_source_ref_string_at(
		quote,
		"prefix",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"prefix\"]",
	)?;
	validate_optional_source_ref_string_at(
		quote,
		"suffix",
		"$.source_ref[\"excerpt_locator\"][\"quote\"][\"suffix\"]",
	)?;

	Ok(())
}

fn validate_source_library_position_locator(position: &Value) -> Result<()> {
	let position = position.as_object().ok_or_else(|| Error::InvalidRequest {
		message: "$.source_ref[\"excerpt_locator\"][\"position\"] must be a JSON object."
			.to_string(),
	})?;
	let start = source_ref_u64(
		position,
		"start",
		"$.source_ref[\"excerpt_locator\"][\"position\"][\"start\"]",
	)?;
	let end = source_ref_u64(
		position,
		"end",
		"$.source_ref[\"excerpt_locator\"][\"position\"][\"end\"]",
	)?;

	if start >= end {
		return Err(Error::InvalidRequest {
			message: "$.source_ref[\"excerpt_locator\"][\"position\"] start must be before end."
				.to_string(),
		});
	}

	Ok(())
}

fn source_ref_u64(source_ref: &Map<String, Value>, key: &str, path: &str) -> Result<u64> {
	source_ref
		.get(key)
		.and_then(Value::as_u64)
		.ok_or_else(|| Error::InvalidRequest { message: format!("{path} must be an integer.") })
}

fn validate_docs_search_l0(req: &DocsSearchL0Request) -> Result<DocsSearchL0Filters> {
	validate_docs_search_l0_query(req)?;

	let filters = parse_docs_search_l0_filters(req)?;
	let ranges = parse_docs_search_l0_ranges(req)?;

	validate_docs_search_l0_temporal_ranges(
		ranges.updated_after.as_ref(),
		ranges.updated_before.as_ref(),
		ranges.ts_gte.as_ref(),
		ranges.ts_lte.as_ref(),
	)?;

	Ok(DocsSearchL0Filters {
		scope: filters.scope,
		status: filters.status,
		doc_type: filters.doc_type,
		sparse_mode: filters.sparse_mode,
		domain: filters.domain,
		repo: filters.repo,
		agent_id: filters.agent_id,
		thread_id: filters.thread_id,
		updated_after: ranges.updated_after,
		updated_before: ranges.updated_before,
		ts_gte: ranges.ts_gte,
		ts_lte: ranges.ts_lte,
	})
}

fn validate_docs_search_l0_query(req: &DocsSearchL0Request) -> Result<()> {
	if req.query.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "query must be non-empty.".to_string() });
	}
	if !english_gate::is_english_natural_language(req.query.as_str()) {
		return Err(Error::NonEnglishInput { field: "$.query".to_string() });
	}

	Ok(())
}

fn parse_docs_search_l0_filters(req: &DocsSearchL0Request) -> Result<DocsSearchL0FiltersParsed> {
	let scope = if let Some(scope) = req.scope.as_ref() {
		let scope = scope.trim();

		if scope.is_empty() {
			return Err(Error::InvalidRequest { message: "scope must be non-empty.".to_string() });
		}
		if !matches!(scope, "agent_private" | "project_shared" | "org_shared") {
			return Err(Error::InvalidRequest { message: "Unknown scope.".to_string() });
		}

		Some(scope.to_string())
	} else {
		None
	};
	let status = req
		.status
		.as_ref()
		.map(|status| status.trim().to_string())
		.filter(|status| !status.is_empty())
		.unwrap_or_else(|| "active".to_string())
		.to_lowercase();
	let status = if DOC_STATUSES.contains(&status.as_str()) {
		status
	} else {
		return Err(Error::InvalidRequest {
			message: "status must be one of: active|deleted.".to_string(),
		});
	};
	let sparse_mode = parse_sparse_mode(req.sparse_mode.as_ref())?;
	let doc_type = if let Some(doc_type) = req.doc_type.as_ref() {
		let doc_type = doc_type.trim();

		if doc_type.is_empty() {
			return Err(Error::InvalidRequest {
				message: "doc_type must be non-empty.".to_string(),
			});
		}

		Some(DocType::parse(doc_type)?)
	} else {
		None
	};
	let domain = req
		.domain
		.as_ref()
		.map(|domain| domain.trim().to_string())
		.filter(|domain| !domain.is_empty());
	let repo =
		req.repo.as_ref().map(|repo| repo.trim().to_string()).filter(|repo| !repo.is_empty());

	if domain.is_some() && doc_type != Some(DocType::Search) {
		return Err(Error::InvalidRequest {
			message: "domain requires doc_type=search.".to_string(),
		});
	}
	if repo.is_some() && doc_type != Some(DocType::Dev) {
		return Err(Error::InvalidRequest { message: "repo requires doc_type=dev.".to_string() });
	}

	let agent_id = req
		.agent_id
		.as_ref()
		.map(|agent_id| agent_id.trim().to_string())
		.filter(|agent_id| !agent_id.is_empty());
	let thread_id = req
		.thread_id
		.as_ref()
		.map(|thread_id| thread_id.trim().to_string())
		.filter(|thread_id| !thread_id.is_empty());

	if thread_id.is_some() && doc_type != Some(DocType::Chat) {
		return Err(Error::InvalidRequest {
			message: "thread_id requires doc_type=chat.".to_string(),
		});
	}

	Ok(DocsSearchL0FiltersParsed {
		scope,
		status,
		doc_type,
		sparse_mode,
		domain,
		repo,
		agent_id,
		thread_id,
	})
}

fn parse_docs_search_l0_ranges(req: &DocsSearchL0Request) -> Result<DocsSearchL0RangesParsed> {
	let updated_after = parse_optional_rfc3339(req.updated_after.as_ref(), "$.updated_after")?;
	let updated_before = parse_optional_rfc3339(req.updated_before.as_ref(), "$.updated_before")?;
	let ts_gte = parse_optional_rfc3339(req.ts_gte.as_ref(), "$.ts_gte")?;
	let ts_lte = parse_optional_rfc3339(req.ts_lte.as_ref(), "$.ts_lte")?;

	Ok(DocsSearchL0RangesParsed { updated_after, updated_before, ts_gte, ts_lte })
}

fn validate_docs_search_l0_temporal_ranges(
	updated_after: Option<&OffsetDateTime>,
	updated_before: Option<&OffsetDateTime>,
	ts_gte: Option<&OffsetDateTime>,
	ts_lte: Option<&OffsetDateTime>,
) -> Result<()> {
	if let (Some(updated_after), Some(updated_before)) = (updated_after, updated_before)
		&& updated_after >= updated_before
	{
		return Err(Error::InvalidRequest {
			message: "updated_after must be earlier than updated_before.".to_string(),
		});
	}
	if let (Some(ts_gte), Some(ts_lte)) = (ts_gte, ts_lte)
		&& ts_gte >= ts_lte
	{
		return Err(Error::InvalidRequest {
			message: "ts_gte must be earlier than ts_lte.".to_string(),
		});
	}

	Ok(())
}

fn parse_sparse_mode(raw: Option<&String>) -> Result<DocsSparseMode> {
	let raw = raw.as_ref().map(|mode| mode.trim().to_lowercase());
	let Some(mode) = raw else {
		return Ok(DocsSparseMode::Auto);
	};
	let mode = mode.as_str();

	match mode {
		"auto" => Ok(DocsSparseMode::Auto),
		"on" => Ok(DocsSparseMode::On),
		"off" => Ok(DocsSparseMode::Off),
		_ => Err(Error::InvalidRequest {
			message: "sparse_mode must be one of: auto|on|off.".to_string(),
		}),
	}
}

fn parse_optional_rfc3339(raw: Option<&String>, path: &str) -> Result<Option<OffsetDateTime>> {
	let Some(raw) = raw else {
		return Ok(None);
	};
	let raw = raw.trim();

	if raw.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{path} must be non-empty.") });
	}

	OffsetDateTime::parse(raw, &Rfc3339).map(Some).map_err(|_| Error::InvalidRequest {
		message: format!("{path} must be an RFC3339 datetime string."),
	})
}

fn find_non_english_path(value: &Value, path: &str) -> Option<String> {
	find_non_english_path_inner(value, path, false)
}

fn find_non_english_path_inner(
	value: &Value,
	path: &str,
	is_identifier_lane: bool,
) -> Option<String> {
	fn has_english_gate(text: &str, is_identifier_lane: bool) -> bool {
		if is_identifier_lane {
			return english_gate::is_english_identifier(text);
		}

		english_gate::is_english_natural_language(text)
	}

	match value {
		Value::String(text) =>
			if !has_english_gate(text, is_identifier_lane) {
				Some(path.to_string())
			} else {
				None
			},
		Value::Array(items) => {
			for (idx, item) in items.iter().enumerate() {
				let child_path = format!("{path}[{idx}]");

				if let Some(found) =
					find_non_english_path_inner(item, &child_path, is_identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		Value::Object(map) => {
			for (key, value) in map.iter() {
				let identifier_lane = is_identifier_lane
					|| matches!(key.as_str(), "ref" | "schema" | "resolver" | "hashes" | "state");
				let child_path = format!("{path}[\"{}\"]", escape_json_path_key(key));

				if let Some(found) =
					find_non_english_path_inner(value, &child_path, identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		_ => None,
	}
}

fn escape_json_path_key(key: &str) -> String {
	key.replace('\\', "\\\\").replace('"', "\\\"")
}

fn load_tokenizer(cfg: &Config) -> Result<Tokenizer> {
	let tokenizer_repo = cfg.chunking.tokenizer_repo.trim();

	if tokenizer_repo.is_empty() {
		return Err(Error::InvalidRequest {
			message: "chunking.tokenizer_repo must be set.".to_string(),
		});
	}

	elf_chunking::load_tokenizer(tokenizer_repo).map_err(|err| Error::InvalidRequest {
		message: format!("failed to load tokenizer: {err}"),
	})
}

fn split_tokens_by_offsets(
	text: &str,
	profile_max_tokens: usize,
	profile_overlap_tokens: usize,
	max_chunks: usize,
	tokenizer: &Tokenizer,
) -> Result<Vec<ByteChunk>> {
	if profile_max_tokens == 0 {
		return Err(Error::InvalidRequest {
			message: "max_tokens must be greater than zero.".to_string(),
		});
	}
	if profile_overlap_tokens >= profile_max_tokens {
		return Err(Error::InvalidRequest {
			message: "overlap_tokens must be less than max_tokens.".to_string(),
		});
	}

	let encoding = tokenizer.encode(text, false).map_err(|err| Error::InvalidRequest {
		message: format!("failed to tokenize content: {err}"),
	})?;
	let offsets = encoding.get_offsets();
	let mut chunks = Vec::new();

	if offsets.is_empty() {
		return Ok(Vec::new());
	}

	let mut chunk_start_token = 0_usize;

	while chunk_start_token < offsets.len() {
		let chunk_end_token = (chunk_start_token + profile_max_tokens).min(offsets.len());
		let (start_offset, end_offset) = {
			let (start, _) = offsets[chunk_start_token];
			let (_, end) = offsets[chunk_end_token.saturating_sub(1)];

			(start, end)
		};
		let chunk_text =
			text.get(start_offset..end_offset).ok_or_else(|| Error::InvalidRequest {
				message: "computed chunk offset is invalid UTF-8 boundary.".to_string(),
			})?;

		chunks.push(ByteChunk {
			chunk_id: Uuid::new_v4(),
			start_offset,
			end_offset,
			text: chunk_text.to_string(),
		});

		if chunk_end_token >= offsets.len() {
			break;
		}
		if chunks.len() >= max_chunks {
			return Err(Error::InvalidRequest {
				message: "doc exceeds max_chunks_per_doc.".to_string(),
			});
		}

		chunk_start_token = chunk_end_token.saturating_sub(profile_overlap_tokens);
	}

	Ok(chunks)
}

fn build_doc_search_filter(
	tenant_id: &str,
	project_id: &str,
	caller_agent_id: &str,
	allowed_scopes: &[String],
	filters: &DocsSearchL0Filters,
) -> Filter {
	let private_scope = "agent_private".to_string();
	let non_private_scopes: Vec<String> =
		allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
	let mut scope_should_conditions = Vec::new();

	if allowed_scopes.iter().any(|scope| scope == "agent_private") {
		let private_filter = Filter::all([
			Condition::matches("scope", private_scope),
			Condition::matches("agent_id", caller_agent_id.to_string()),
		]);

		scope_should_conditions.push(Condition::from(private_filter));
	}
	if !non_private_scopes.is_empty() {
		scope_should_conditions.push(Condition::matches("scope", non_private_scopes));
	}

	let scope_min_should = if scope_should_conditions.is_empty() {
		None
	} else {
		Some(MinShould { min_count: 1, conditions: scope_should_conditions })
	};
	let mut project_or_org_branches = vec![Condition::from(Filter {
		must: vec![Condition::matches("project_id", project_id.to_string())],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: scope_min_should,
	})];

	if allowed_scopes.iter().any(|scope| scope == "org_shared") {
		let org_filter = Filter::all([
			Condition::matches("project_id", ORG_PROJECT_ID.to_string()),
			Condition::matches("scope", "org_shared".to_string()),
		]);

		project_or_org_branches.push(Condition::from(org_filter));
	}

	Filter {
		must: {
			let mut must = vec![
				Condition::matches("tenant_id", tenant_id.to_string()),
				Condition::matches("status", filters.status.clone()),
			];

			if let Some(scope) = filters.scope.as_ref() {
				must.push(Condition::matches("scope", scope.to_string()));
			}
			if let Some(doc_type) = filters.doc_type.as_ref() {
				must.push(Condition::matches("doc_type", doc_type.as_str().to_string()));
			}
			if let Some(domain) = filters.domain.as_ref() {
				must.push(Condition::matches("domain", domain.to_string()));
			}
			if let Some(repo) = filters.repo.as_ref() {
				must.push(Condition::matches("repo", repo.to_string()));
			}
			if let Some(agent_id) = filters.agent_id.as_ref() {
				must.push(Condition::matches("agent_id", agent_id.to_string()));
			}
			if let Some(thread_id) = filters.thread_id.as_ref() {
				must.push(Condition::matches("thread_id", thread_id.to_string()));
			}
			if let Some(datetime_filter) = datetime_filter_range(
				filters.updated_after.as_ref(),
				filters.updated_before.as_ref(),
			) {
				must.push(datetime_filter);
			}
			if let Some(datetime_filter) =
				doc_ts_filter_range(filters.ts_gte.as_ref(), filters.ts_lte.as_ref())
			{
				must.push(datetime_filter);
			}

			must
		},
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: Some(MinShould { min_count: 1, conditions: project_or_org_branches }),
	}
}

fn datetime_filter_range(
	updated_after: Option<&OffsetDateTime>,
	updated_before: Option<&OffsetDateTime>,
) -> Option<Condition> {
	let gt = updated_after.map(|updated_after| Timestamp {
		seconds: updated_after.unix_timestamp(),
		nanos: updated_after.nanosecond() as i32,
	});
	let lt = updated_before.map(|updated_before| Timestamp {
		seconds: updated_before.unix_timestamp(),
		nanos: updated_before.nanosecond() as i32,
	});

	if gt.is_none() && lt.is_none() {
		return None;
	}

	Some(Condition::datetime_range("updated_at", DatetimeRange { lt, gt, gte: None, lte: None }))
}

fn doc_ts_filter_range(
	ts_gte: Option<&OffsetDateTime>,
	ts_lte: Option<&OffsetDateTime>,
) -> Option<Condition> {
	let gte = ts_gte.map(|ts_gte| Timestamp {
		seconds: ts_gte.unix_timestamp(),
		nanos: ts_gte.nanosecond() as i32,
	});
	let lte = ts_lte.map(|ts_lte| Timestamp {
		seconds: ts_lte.unix_timestamp(),
		nanos: ts_lte.nanosecond() as i32,
	});

	if gte.is_none() && lte.is_none() {
		return None;
	}

	Some(Condition::datetime_range("doc_ts", DatetimeRange { lt: None, gt: None, gte, lte }))
}

fn doc_read_allowed(
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	owner_agent_id: &str,
	scope: &str,
) -> bool {
	if !allowed_scopes.iter().any(|s| s == scope) {
		return false;
	}
	if scope == "agent_private" {
		return owner_agent_id == requester_agent_id;
	}
	if owner_agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&SharedSpaceGrantKey {
		scope: scope.to_string(),
		space_owner_agent_id: owner_agent_id.to_string(),
	})
}

fn parse_scored_point_uuid_id(point: &ScoredPoint) -> Result<Uuid> {
	let id = point
		.id
		.as_ref()
		.ok_or_else(|| Error::Qdrant { message: "Qdrant returned item without id.".to_string() })?;

	match id.point_id_options.as_ref() {
		Some(PointIdOptions::Uuid(s)) => Uuid::parse_str(s.as_str())
			.map_err(|_| Error::Qdrant { message: "Qdrant returned invalid uuid id.".to_string() }),
		Some(other) => Err(Error::Qdrant {
			message: format!("Qdrant returned unsupported id type: {other:?}."),
		}),
		None => Err(Error::Qdrant { message: "Qdrant returned item with missing id.".to_string() }),
	}
}

fn truncate_bytes(text: &str, max: usize) -> String {
	if text.len() <= max {
		return text.to_string();
	}

	let mut cut = max;

	while cut > 0 && !text.is_char_boundary(cut) {
		cut -= 1;
	}

	text.get(0..cut).unwrap_or("").to_string()
}

fn locate_quote(text: &str, quote: &TextQuoteSelector) -> Option<(usize, usize)> {
	let prefix = quote.prefix.as_deref().unwrap_or("");
	let suffix = quote.suffix.as_deref().unwrap_or("");

	for (start, _) in text.match_indices(quote.exact.as_str()) {
		let end = start + quote.exact.len();

		if !text[..start].ends_with(prefix) {
			continue;
		}
		if !text[end..].starts_with(suffix) {
			continue;
		}

		return Some((start, end));
	}

	None
}

fn bounded_window(
	match_start: usize,
	match_end: usize,
	text: &str,
	max_bytes: usize,
) -> (usize, usize) {
	let len = text.len();
	let match_center = match_start.saturating_add(match_end.saturating_sub(match_start) / 2);
	let half = max_bytes / 2;
	let mut start = match_center.saturating_sub(half);
	let mut end = (start + max_bytes).min(len);

	if end - start < max_bytes && start > 0 {
		start = start.saturating_sub(max_bytes - (end - start));
	}

	while start < len && !text.is_char_boundary(start) {
		start += 1;
	}
	while end > start && !text.is_char_boundary(end) {
		end -= 1;
	}

	(start, end)
}

fn docs_search_sparse_enabled(mode: DocsSparseMode, query: &str) -> bool {
	match mode {
		DocsSparseMode::Auto => should_enable_sparse_auto(query),
		DocsSparseMode::On => true,
		DocsSparseMode::Off => false,
	}
}

fn should_enable_sparse_auto(query: &str) -> bool {
	let trimmed = query.trim();

	if trimmed.is_empty() {
		return false;
	}
	if trimmed.contains("://")
		|| trimmed.contains('/')
		|| trimmed.contains('\\')
		|| trimmed.contains('?')
	{
		return true;
	}

	let has_mixed_alpha_num = trimmed.split_whitespace().any(|token| {
		token.chars().any(|ch| ch.is_ascii_alphabetic())
			&& token.chars().any(|ch| ch.is_ascii_digit())
	});
	let special_count = trimmed
		.chars()
		.filter(|ch| !(ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || *ch == '_'))
		.count();
	let compact_hex_like = {
		let compact = trimmed.chars().filter(|ch| !ch.is_ascii_whitespace()).collect::<String>();

		compact.len() >= 12 && compact.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
	};

	special_count >= 2 || compact_hex_like || (has_mixed_alpha_num && trimmed.len() > 12)
}

async fn load_docs_excerpt_context(
	cfg: &Config,
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
	doc_id: Uuid,
) -> Result<DocDocument> {
	let allowed_scopes = search::resolve_read_profile_scopes(cfg, read_profile)?;
	let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
	let shared_grants = access::load_shared_read_grants_with_org_shared(
		pool,
		tenant_id,
		project_id,
		agent_id,
		org_shared_allowed,
	)
	.await?;
	let doc = load_doc_document_for_read(pool, doc_id, tenant_id, project_id)
		.await?
		.ok_or_else(|| Error::NotFound { message: "Doc not found.".to_string() })?;

	if doc.status != "active"
		|| !doc_read_allowed(
			agent_id,
			&allowed_scopes,
			&shared_grants,
			doc.agent_id.as_str(),
			doc.scope.as_str(),
		) {
		return Err(Error::NotFound { message: "Doc not found.".to_string() });
	}

	Ok(doc)
}

async fn docs_excerpts_resolve_windowed_match(
	pool: &PgPool,
	doc: &DocDocument,
	req: &DocsExcerptsGetRequest,
	level_max: usize,
	trajectory: &mut DocTrajectoryBuilder,
	verified: &mut bool,
	verification_errors: &mut Vec<String>,
) -> Result<DocExcerptRange> {
	let DocExcerptMatch { selector_kind, match_start_offset, match_end_offset } =
		docs_excerpts_resolve_match(pool, doc, req, verified, verification_errors).await?;

	trajectory.push(
		"match_resolution",
		serde_json::json!({
			"selector_kind": selector_kind.as_str(),
			"match_start": match_start_offset,
			"match_end": match_end_offset,
		}),
	);

	let (start_offset, end_offset) =
		bounded_window(match_start_offset, match_end_offset, doc.content.as_str(), level_max);

	trajectory.push(
		"window_projection",
		serde_json::json!({
			"window_start": start_offset,
			"window_end": end_offset,
			"content_len": doc.content.len(),
		}),
	);

	Ok(DocExcerptRange {
		selector_kind,
		match_start_offset,
		match_end_offset,
		start_offset,
		end_offset,
	})
}

async fn docs_excerpts_resolve_match(
	pool: &PgPool,
	doc: &DocDocument,
	req: &DocsExcerptsGetRequest,
	verified: &mut bool,
	verification_errors: &mut Vec<String>,
) -> Result<DocExcerptMatch> {
	let (match_start_offset, match_end_offset, selector_kind) =
		resolve_excerpts_match_range(pool, doc, req, verified, verification_errors).await?;

	Ok(DocExcerptMatch { selector_kind, match_start_offset, match_end_offset })
}

async fn load_doc_document_for_read(
	executor: impl PgExecutor<'_>,
	doc_id: Uuid,
	tenant_id: &str,
	project_id: &str,
) -> Result<Option<DocDocument>> {
	let row: Option<DocDocument> = sqlx::query_as::<_, DocDocument>(
		"\
SELECT
	doc_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	doc_type,
	status,
	title,
	COALESCE(source_ref, '{}'::jsonb) AS source_ref,
	content,
	content_bytes,
	content_hash,
	created_at,
	updated_at
FROM doc_documents
WHERE doc_id = $1
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )
LIMIT 1",
	)
	.bind(doc_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ORG_PROJECT_ID)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

async fn resolve_excerpts_match_range(
	pool: &PgPool,
	doc: &DocDocument,
	req: &DocsExcerptsGetRequest,
	verified: &mut bool,
	verification_errors: &mut Vec<String>,
) -> Result<(usize, usize, ExcerptsSelectorKind)> {
	if let Some(chunk_id) = req.chunk_id {
		let chunk = docs::get_doc_chunk(pool, chunk_id).await?;
		let Some(chunk) = chunk else {
			return Err(Error::NotFound { message: "Chunk not found.".to_string() });
		};

		if chunk.doc_id != doc.doc_id {
			return Err(Error::NotFound { message: "Chunk not found.".to_string() });
		}

		return Ok((
			chunk.start_offset.max(0) as usize,
			chunk.end_offset.max(0) as usize,
			ExcerptsSelectorKind::ChunkId,
		));
	}
	if let Some(quote) = req.quote.as_ref() {
		return Ok(match locate_quote(&doc.content, quote) {
			Some((s, e)) => (s, e, ExcerptsSelectorKind::Quote),
			None => {
				*verified = false;

				verification_errors.push("QUOTE_SELECTOR_NOT_FOUND".to_string());

				if let Some(pos) = req.position.as_ref() {
					(
						pos.start.min(doc.content.len()),
						pos.end.min(doc.content.len()),
						ExcerptsSelectorKind::Position,
					)
				} else {
					return Err(Error::NotFound {
						message: "Selector did not match document.".to_string(),
					});
				}
			},
		});
	}
	if let Some(pos) = req.position.as_ref() {
		return Ok((
			pos.start.min(doc.content.len()),
			pos.end.min(doc.content.len()),
			ExcerptsSelectorKind::Position,
		));
	}

	Err(Error::InvalidRequest {
		message: "One of chunk_id, quote, or position is required.".to_string(),
	})
}

async fn run_doc_fusion_query(
	client: &Qdrant,
	collection: &str,
	query_text: &str,
	vector: &[f32],
	filter: &Filter,
	sparse_mode: DocsSparseMode,
	candidate_k: u32,
) -> Result<Vec<ScoredPoint>> {
	let sparse_enabled = docs_search_sparse_enabled(sparse_mode, query_text);
	let dense_prefetch = PrefetchQueryBuilder::default()
		.query(Query::new_nearest(vector.to_vec()))
		.using(DENSE_VECTOR_NAME)
		.filter(filter.clone())
		.limit(candidate_k as u64);
	let mut search = QueryPointsBuilder::new(collection.to_string());

	search = search.add_prefetch(dense_prefetch);

	if sparse_enabled {
		let bm25_prefetch = PrefetchQueryBuilder::default()
			.query(Query::new_nearest(Document::new(query_text.to_string(), BM25_MODEL)))
			.using(BM25_VECTOR_NAME)
			.filter(filter.clone())
			.limit(candidate_k as u64);

		search = search.add_prefetch(bm25_prefetch);
	}

	let search = search.with_payload(false).query(Fusion::Rrf).limit(candidate_k as u64);
	let response =
		client.query(search).await.map_err(|err| Error::Qdrant { message: err.to_string() })?;

	Ok(response.result)
}

async fn load_doc_search_rows(
	executor: impl PgExecutor<'_>,
	tenant_id: &str,
	project_id: &str,
	status: &str,
	chunk_ids: &[Uuid],
) -> Result<HashMap<Uuid, DocSearchRow>> {
	if chunk_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows: Vec<DocSearchRow> = sqlx::query_as(
		"\
SELECT
	c.chunk_id,
	c.doc_id,
	d.scope,
	d.doc_type,
	d.project_id,
	d.agent_id,
	d.updated_at,
	d.content_hash,
	c.chunk_hash,
	c.start_offset,
	c.end_offset,
	c.chunk_text
FROM doc_chunks c
JOIN doc_documents d ON d.doc_id = c.doc_id
WHERE c.chunk_id = ANY($1)
  AND d.tenant_id = $2
  AND d.status = $4
  AND (
    d.project_id = $3
    OR (d.project_id = $5 AND d.scope = 'org_shared')
  )",
	)
	.bind(chunk_ids)
	.bind(tenant_id)
	.bind(project_id)
	.bind(status)
	.bind(ORG_PROJECT_ID)
	.fetch_all(executor)
	.await?;
	let mut map = HashMap::with_capacity(rows.len());

	for row in rows {
		map.insert(row.chunk_id, row);
	}

	Ok(map)
}

#[cfg(test)]
mod tests {
	use ahash::AHashMap;
	use qdrant_client::qdrant::{
		DatetimeRange, Filter, condition::ConditionOneOf, r#match::MatchValue,
	};
	use time::{OffsetDateTime, format_description::well_known::Rfc3339};
	use tokenizers::{
		Tokenizer, models::wordlevel::WordLevel, pre_tokenizers::whitespace::Whitespace,
	};
	use uuid::Uuid;

	use crate::docs::{
		self, DocType, DocsPutRequest, DocsSearchL0Filters, DocsSearchL0Request, DocsSparseMode,
		Error,
	};
	use elf_domain::writegate::{WritePolicy, WritePolicyAudit, WriteRedactionResult, WriteSpan};
	use elf_storage::models::DocChunk;

	const TENANT_ID: &str = "tenant";
	const PROJECT_ID: &str = "project";

	fn test_request_with_query(query: &str) -> DocsSearchL0Request {
		DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: query.to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		}
	}

	fn first_datetime_range(filter: &Filter, key: &str) -> Option<DatetimeRange> {
		for condition in &filter.must {
			if let Some(ConditionOneOf::Field(field)) = condition.condition_one_of.as_ref() {
				if field.key != key {
					continue;
				}

				if let Some(range) = field.datetime_range.as_ref() {
					return Some(*range);
				}
			}
		}

		None
	}

	fn first_match_value(filter: &Filter, key: &str) -> Option<String> {
		for condition in &filter.must {
			if let Some(ConditionOneOf::Field(field)) = condition.condition_one_of.as_ref() {
				if field.key != key {
					continue;
				}

				if let Some(r#match) = field.r#match.as_ref() {
					let Some(match_value) = r#match.match_value.as_ref() else {
						continue;
					};

					return match match_value {
						MatchValue::Keyword(value) => Some(value.clone()),
						_ => None,
					};
				}
			}
		}

		None
	}

	fn test_tokenizer() -> Tokenizer {
		let mut vocab = AHashMap::new();

		vocab.insert("alpha".to_string(), 1_u32);
		vocab.insert("beta".to_string(), 2_u32);
		vocab.insert("charlie".to_string(), 3_u32);
		vocab.insert("delta".to_string(), 4_u32);
		vocab.insert("<unk>".to_string(), 0_u32);

		let model = WordLevel::builder()
			.vocab(vocab)
			.unk_token("<unk>".to_string())
			.build()
			.expect("Failed to build test tokenizer.");
		let mut tokenizer = Tokenizer::new(model);

		tokenizer.with_pre_tokenizer(Some(Whitespace));

		tokenizer
	}

	#[test]
	fn doc_type_parses_and_serializes() {
		let encoded =
			serde_json::to_string(&DocType::Knowledge).expect("Expected DocType serialization.");
		let parsed =
			serde_json::from_str::<DocType>("\"knowledge\"").expect("Expected parse to succeed.");
		let invalid: Result<DocType, _> = serde_json::from_str("\"invalid\"");

		assert_eq!(encoded, "\"knowledge\"");
		assert_eq!(parsed, DocType::Knowledge);
		assert!(invalid.is_err());
	}

	#[test]
	fn docs_search_l0_requires_chat_doc_type_for_thread_id() {
		let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "thread".to_string(),
			scope: None,
			status: None,
			doc_type: Some("search".to_string()),
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: Some("thread-1".to_string()),
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect_err("Expected thread_id to require doc_type=chat.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("thread_id requires")),
			other => panic!("Unexpected error: {other:?}"),
		}

		docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "thread".to_string(),
			scope: None,
			status: None,
			doc_type: Some("chat".to_string()),
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: Some("thread-1".to_string()),
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect("Expected thread_id filter to be accepted for chat.");
	}

	#[test]
	fn validate_docs_put_rejects_invalid_doc_type() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "invalid",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected invalid doc_type to be rejected.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("doc_type")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn resolve_doc_chunking_profile_is_deterministic_by_doc_type() {
		let small = docs::resolve_doc_chunking_profile(DocType::Chat);

		assert_eq!(small.max_tokens, 1_024);
		assert_eq!(small.overlap_tokens, 128);

		let default = docs::resolve_doc_chunking_profile(DocType::Knowledge);

		assert_eq!(default.max_tokens, 2_048);
		assert_eq!(default.overlap_tokens, 256);
	}

	#[test]
	fn validate_docs_search_l0_defaults_status_and_filters_dates() {
		let filters = docs::validate_docs_search_l0(&test_request_with_query("hello world"))
			.expect("valid request");

		assert_eq!(filters.status, "active");

		let bad_dates = DocsSearchL0Request {
			updated_after: Some("2026-02-25T12:00:00Z".to_string()),
			updated_before: Some("2026-02-25T11:00:00Z".to_string()),
			sparse_mode: None,
			domain: None,
			repo: None,
			..test_request_with_query("status")
		};
		let err = docs::validate_docs_search_l0(&bad_dates)
			.expect_err("Expected bad date order to be rejected.");

		match err {
			Error::InvalidRequest { message } => {
				assert!(message.contains("earlier"));
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_search_l0_rejects_invalid_status() {
		let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "status".to_string(),
			scope: None,
			status: Some("archived".to_string()),
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect_err("Expected invalid status to be rejected.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("status")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_search_l0_rejects_invalid_datetime_format() {
		let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "status".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: Some("2026-02-25T12:00:00".to_string()),
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect_err("Expected invalid RFC3339 datetime to be rejected.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("RFC3339")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn build_doc_search_filter_applies_status_and_requested_filters() {
		let filters = DocsSearchL0Filters {
			scope: Some("project_shared".to_string()),
			status: "deleted".to_string(),
			doc_type: Some(DocType::Chat),
			sparse_mode: DocsSparseMode::Auto,
			domain: None,
			repo: None,
			agent_id: Some("owner".to_string()),
			thread_id: Some("thread-7".to_string()),
			updated_after: Some(
				OffsetDateTime::parse("2026-02-20T00:00:00Z", &Rfc3339)
					.expect("Invalid timestamp."),
			),
			updated_before: Some(
				OffsetDateTime::parse("2026-02-28T00:00:00Z", &Rfc3339)
					.expect("Invalid timestamp."),
			),
			ts_gte: Some(
				OffsetDateTime::parse("2026-01-01T00:00:00Z", &Rfc3339)
					.expect("Invalid timestamp."),
			),
			ts_lte: Some(
				OffsetDateTime::parse("2026-12-31T00:00:00Z", &Rfc3339)
					.expect("Invalid timestamp."),
			),
		};
		let filter = super::build_doc_search_filter(
			TENANT_ID,
			PROJECT_ID,
			"requester",
			&["agent_private".to_string(), "project_shared".to_string()],
			&filters,
		);

		assert_eq!(first_match_value(&filter, "tenant_id").as_deref(), Some("tenant"));
		assert_eq!(first_match_value(&filter, "status").as_deref(), Some("deleted"));
		assert_eq!(first_match_value(&filter, "scope").as_deref(), Some("project_shared"));
		assert_eq!(first_match_value(&filter, "doc_type").as_deref(), Some("chat"));
		assert_eq!(first_match_value(&filter, "agent_id").as_deref(), Some("owner"));
		assert_eq!(first_match_value(&filter, "thread_id").as_deref(), Some("thread-7"));
		assert_eq!(first_match_value(&filter, "domain").as_deref(), None);
		assert_eq!(first_match_value(&filter, "repo").as_deref(), None);

		let datetime_range = first_datetime_range(&filter, "updated_at")
			.expect("Expected datetime filter for updated_at.");
		let after =
			OffsetDateTime::parse("2026-02-20T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");
		let before =
			OffsetDateTime::parse("2026-02-28T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");
		let lt = datetime_range.lt.as_ref().expect("Expected datetime filter .lt value.");
		let gt = datetime_range.gt.as_ref().expect("Expected datetime filter .gt value.");

		assert_eq!(lt.seconds, before.unix_timestamp());
		assert_eq!(lt.nanos, before.nanosecond() as i32);
		assert_eq!(gt.seconds, after.unix_timestamp());
		assert_eq!(gt.nanos, after.nanosecond() as i32);
		assert!(datetime_range.gte.is_none());
		assert!(datetime_range.lte.is_none());

		let doc_ts_range =
			first_datetime_range(&filter, "doc_ts").expect("Expected datetime filter for doc_ts.");
		let gte = doc_ts_range.gte.as_ref().expect("Expected datetime filter .gte value.");
		let lte = doc_ts_range.lte.as_ref().expect("Expected datetime filter .lte value.");
		let doc_ts_gte =
			OffsetDateTime::parse("2026-01-01T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");
		let doc_ts_lte =
			OffsetDateTime::parse("2026-12-31T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");

		assert_eq!(gte.seconds, doc_ts_gte.unix_timestamp());
		assert_eq!(gte.nanos, doc_ts_gte.nanosecond() as i32);
		assert_eq!(lte.seconds, doc_ts_lte.unix_timestamp());
		assert_eq!(lte.nanos, doc_ts_lte.nanosecond() as i32);
		assert!(doc_ts_range.gt.is_none());
		assert!(doc_ts_range.lt.is_none());
	}

	#[test]
	fn validate_docs_search_l0_rejects_invalid_doc_ts_order() {
		let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "status".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: Some("2026-02-25T12:00:00Z".to_string()),
			ts_lte: Some("2026-02-25T11:00:00Z".to_string()),
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect_err("Expected bad doc_ts order to be rejected.");

		match err {
			Error::InvalidRequest { message } => {
				assert!(message.contains("earlier"));
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_search_l0_rejects_invalid_sparse_mode() {
		let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "status".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: Some("invalid".to_string()),
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect_err("Expected invalid sparse mode to be rejected.");

		match err {
			Error::InvalidRequest { message } => {
				assert!(message.contains("sparse_mode"));
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_search_l0_rejects_domain_without_doc_type_search() {
		let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "status".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: Some("example.com".to_string()),
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect_err("Expected domain without doc_type=search to be rejected.");

		match err {
			Error::InvalidRequest { message } => {
				assert!(message.contains("doc_type=search"));
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_search_l0_rejects_repo_without_doc_type_dev() {
		let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			caller_agent_id: "agent".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "status".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: Some("hack-ink/ELF".to_string()),
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			top_k: None,
			candidate_k: None,
			explain: None,
		})
		.expect_err("Expected repo without doc_type=dev to be rejected.");

		match err {
			Error::InvalidRequest { message } => {
				assert!(message.contains("doc_type=dev"));
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_search_l0_default_sparse_mode() {
		let filters = docs::validate_docs_search_l0(&test_request_with_query("status"))
			.expect("valid request");

		assert!(matches!(filters.sparse_mode, DocsSparseMode::Auto));
	}

	#[test]
	fn should_enable_sparse_auto_uses_symbol_cues() {
		assert!(super::should_enable_sparse_auto("https://example.com/search?q=abc"));
		assert!(!super::should_enable_sparse_auto("how to debug a timeout"));
	}

	#[test]
	fn excerpt_level_max_supports_l0_and_rejects_unknown_level() {
		assert_eq!(
			super::excerpt_level_max("L0").expect("Expected L0 to be supported."),
			super::DEFAULT_L0_MAX_BYTES
		);
		assert!(super::excerpt_level_max("L3").is_err());
	}

	#[test]
	fn validate_docs_put_rejects_missing_source_ref() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Knowledge.as_str().to_string()),
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({"schema":"doc_source_ref/v1", "doc_type":"knowledge"}),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected missing source_ref.ts to be rejected.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("source_ref[\"ts\"]")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_rejects_non_object_source_ref() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: None,
			write_policy: None,
			source_ref: serde_json::json!("legacy-shape"),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected non-object source_ref to be rejected.");

		match err {
			Error::InvalidRequest { message } => {
				assert!(message.contains("source_ref must be a JSON object"))
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_rejects_mismatched_request_and_source_ref_doc_type() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Chat.as_str().to_string()),
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected mismatched doc_type to be rejected.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("match")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_rejects_wrong_source_ref_schema() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "note_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected wrong source_ref.schema to be rejected.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("doc_source_ref/v1")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_rejects_chat_source_ref_with_missing_thread_metadata() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Chat.as_str().to_string()),
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "chat",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected chat source_ref to require thread_id/role.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("thread_id")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_rejects_search_source_ref_with_missing_domain() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Search.as_str().to_string()),
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "search",
				"ts": "2026-02-25T12:00:00Z",
				"query": "test",
				"url": "https://example.com",
			}),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected search source_ref to require domain.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("domain")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_rejects_dev_source_ref_with_multiple_identifiers() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Dev.as_str().to_string()),
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "dev",
				"ts": "2026-02-25T12:00:00Z",
				"repo": "hack-ink/ELF",
				"commit_sha": "9f0a3f4c4eb58bfcf4a5f4f9d0c7be0e13c2f8d19",
				"issue_number": 123,
			}),
			content: "Hello world.".to_string(),
		})
		.expect_err("Expected dev source_ref to enforce exactly one identifier field.");

		match err {
			Error::InvalidRequest { message } => {
				assert!(message.contains("exactly one of commit_sha, pr_number, or issue_number"))
			},
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_uses_source_ref_doc_type_when_request_doc_type_is_absent() {
		let resolved_doc_type = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: None,
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "chat",
				"ts": "2026-02-25T12:00:00Z",
				"thread_id": "thread-1",
				"role": "assistant"
			}),
			content: "Hello world.".to_string(),
		})
		.expect("Expected valid source_ref to resolve doc_type.");

		assert_eq!(resolved_doc_type.doc_type, DocType::Chat);
	}

	#[test]
	fn validate_docs_put_accepts_source_library_article_metadata() {
		let validated = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Knowledge.as_str().to_string()),
			title: Some("Saved article".to_string()),
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"source_kind": "article",
				"canonical_uri": "https://example.com/research/source-library",
				"captured_at": "2026-02-25T12:10:00Z",
				"source_created_at": "2026-02-24T09:00:00Z",
				"trust_label": "public_web",
				"author": "Example Author",
				"handle": "example-author",
				"excerpt_locator": {
					"quote": {
						"exact": "Source libraries preserve long-form evidence."
					},
					"position": {
						"start": 0,
						"end": 48
					}
				}
			}),
			content: "Source libraries preserve long-form evidence. Agents can hydrate exact excerpts later.".to_string(),
		})
		.expect("Expected source library metadata to be accepted.");

		assert_eq!(validated.doc_type, DocType::Knowledge);
	}

	#[test]
	fn source_capture_metadata_uses_stable_record_and_span_ids() {
		let now = OffsetDateTime::parse("2026-02-25T12:15:00Z", &Rfc3339)
			.expect("Expected test timestamp to parse.");
		let source_ref = serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"source_kind": "article",
			"canonical_uri": "https://example.com/research/source-library",
			"captured_at": "2026-02-25T12:10:00Z",
			"trust_label": "public_web",
		});
		let source_ref = source_ref.as_object().expect("Expected source_ref object.");
		let content_hash = "doc-content-hash";
		let doc_id = super::source_record_id_for(
			TENANT_ID,
			PROJECT_ID,
			"owner",
			"project_shared",
			DocType::Knowledge,
			source_ref,
			content_hash,
		);
		let repeated_doc_id = super::source_record_id_for(
			TENANT_ID,
			PROJECT_ID,
			"owner",
			"project_shared",
			DocType::Knowledge,
			source_ref,
			content_hash,
		);
		let chunk_id = super::doc_chunk_id_for(doc_id, 0);
		let chunk = DocChunk {
			chunk_id,
			doc_id,
			chunk_index: 0,
			start_offset: 0,
			end_offset: 42,
			chunk_text: "Source libraries preserve long-form evidence.".to_string(),
			chunk_hash: "chunk-content-hash".to_string(),
			created_at: now,
		};
		let capture = super::build_source_capture_summary(super::SourceCaptureSummaryInput {
			doc_id,
			source_ref,
			doc_type: DocType::Knowledge,
			scope: "project_shared",
			title: Some("Saved article"),
			content_hash,
			raw_content_hash: "raw-content-hash",
			now,
			chunks: &[chunk],
			write_policy_audit: None,
		})
		.expect("Expected source capture summary.");

		assert_eq!(doc_id, repeated_doc_id);
		assert_eq!(capture.schema, "doc_source_capture/v1");
		assert_eq!(capture.source_record_id, doc_id);
		assert_eq!(capture.origin, "https://example.com/research/source-library");
		assert_eq!(capture.captured_at, "2026-02-25T12:10:00Z");
		assert_eq!(capture.content_hash, content_hash);
		assert_eq!(capture.visibility_scope, "project_shared");
		assert_eq!(capture.title.as_deref(), Some("Saved article"));
		assert_eq!(capture.source_type, "article");
		assert_eq!(capture.source_spans.len(), 1);
		assert_eq!(capture.source_spans[0].schema, "doc_source_span/v1");
		assert_eq!(capture.source_spans[0].chunk_id, Some(chunk_id));
		assert_eq!(capture.source_spans[0].status, "captured");
		assert_eq!(capture.source_spans[0].reason_code, None);
		assert_eq!(capture.source_spans[0].start_offset, 0);
		assert_eq!(capture.source_spans[0].end_offset, 42);
		assert_eq!(
			capture.source_spans[0].span_id,
			super::source_span_id(content_hash, 0, 42, "captured")
		);
	}

	#[test]
	fn normalized_source_ref_records_policy_span_reasons() {
		let now = OffsetDateTime::parse("2026-02-25T12:15:00Z", &Rfc3339)
			.expect("Expected test timestamp to parse.");
		let source_ref = serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"uri": "file:///tmp/source.txt",
		});
		let source_ref_map = source_ref.as_object().expect("Expected source_ref object.");
		let audit = WritePolicyAudit {
			exclusions: vec![WriteSpan { start: 6, end: 12 }],
			redactions: vec![WriteRedactionResult {
				span: WriteSpan { start: 20, end: 30 },
				replacement: "[redacted]".to_string(),
			}],
		};
		let doc_id = super::source_record_id_for(
			TENANT_ID,
			PROJECT_ID,
			"owner",
			"project_shared",
			DocType::Knowledge,
			source_ref_map,
			"stored-hash",
		);
		let capture = super::build_source_capture_summary(super::SourceCaptureSummaryInput {
			doc_id,
			source_ref: source_ref_map,
			doc_type: DocType::Knowledge,
			scope: "project_shared",
			title: None,
			content_hash: "stored-hash",
			raw_content_hash: "raw-hash",
			now,
			chunks: &[],
			write_policy_audit: Some(&audit),
		})
		.expect("Expected source capture summary.");
		let normalized = super::normalize_source_ref_for_capture(source_ref, &capture)
			.expect("Expected normalized source_ref");

		assert_eq!(capture.policy_spans.len(), 2);
		assert_eq!(capture.policy_spans[0].status, "excluded");
		assert_eq!(capture.policy_spans[0].reason_code.as_deref(), Some("WRITE_POLICY_EXCLUSION"));
		assert_eq!(capture.policy_spans[1].status, "redacted");
		assert_eq!(capture.policy_spans[1].reason_code.as_deref(), Some("WRITE_POLICY_REDACTION"));
		assert_eq!(normalized["source_record_id"], doc_id.to_string());
		assert_eq!(normalized["origin"], "file:///tmp/source.txt");
		assert_eq!(normalized["captured_at"], "2026-02-25T12:15:00Z");
		assert_eq!(normalized["content_hash"], "stored-hash");
		assert_eq!(normalized["visibility_scope"], "project_shared");
		assert_eq!(normalized["source_type"], "knowledge");
		assert_eq!(normalized["policy_spans"][0]["reason_code"], "WRITE_POLICY_EXCLUSION");
		assert_eq!(normalized["policy_spans"][1]["reason_code"], "WRITE_POLICY_REDACTION");
	}

	#[test]
	fn validate_docs_put_rejects_incomplete_source_library_metadata() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Knowledge.as_str().to_string()),
			title: Some("Saved article".to_string()),
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"source_kind": "article",
				"captured_at": "2026-02-25T12:10:00Z",
				"trust_label": "public_web"
			}),
			content: "Source libraries preserve long-form evidence.".to_string(),
		})
		.expect_err("Expected canonical_uri to be required for source library metadata.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("canonical_uri")),
			other => panic!("Unexpected error: {other:?}"),
		}

		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Knowledge.as_str().to_string()),
			title: Some("Saved thread".to_string()),
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"source_kind": "social_thread",
				"canonical_uri": "https://example.com/thread/123",
				"captured_at": "2026-02-25T12:10:00Z",
				"trust_label": "public_web"
			}),
			content: "The thread says source libraries need social captures.".to_string(),
		})
		.expect_err("Expected social_thread source_kind to require chat doc_type.");

		match err {
			Error::InvalidRequest { message } =>
				assert!(message.contains("requires doc_type=chat")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn docs_l0_pointer_carries_hashes_and_position_locator() {
		let now = OffsetDateTime::parse("2026-02-25T12:00:00Z", &Rfc3339)
			.expect("Expected test timestamp to parse.");
		let row = super::DocSearchRow {
			chunk_id: Uuid::parse_str("11111111-1111-4111-8111-111111111111")
				.expect("Expected chunk UUID."),
			doc_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222")
				.expect("Expected doc UUID."),
			scope: "project_shared".to_string(),
			doc_type: "knowledge".to_string(),
			project_id: "project".to_string(),
			agent_id: "agent".to_string(),
			updated_at: now,
			content_hash: "doc-hash".to_string(),
			chunk_hash: "chunk-hash".to_string(),
			start_offset: 12,
			end_offset: 64,
			chunk_text: "Source libraries preserve long-form evidence.".to_string(),
		};
		let pointer = super::build_docs_l0_pointer(&row, row.chunk_id);

		assert_eq!(pointer.schema, "source_ref/v1");
		assert_eq!(pointer.resolver, "elf_doc_ext/v1");
		assert_eq!(pointer.hashes.content_hash, "doc-hash");
		assert_eq!(pointer.hashes.chunk_hash, "chunk-hash");
		assert_eq!(pointer.reference.source_record_id, row.doc_id);
		assert_eq!(pointer.reference.source_span_id, pointer.locator.span_id);
		assert_eq!(pointer.locator.position.start, 12);
		assert_eq!(pointer.locator.position.end, 64);
		assert_eq!(pointer.locator.span_id, super::source_span_id("doc-hash", 12, 64, "captured"));
		assert_eq!(pointer.state.content_hash, pointer.hashes.content_hash);
		assert_eq!(pointer.state.chunk_hash, pointer.hashes.chunk_hash);
	}

	#[test]
	fn validate_docs_put_applies_write_policy_and_includes_audit() {
		let validated = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Knowledge.as_str().to_string()),
			title: None,
			write_policy: Some(WritePolicy {
				exclusions: vec![WriteSpan { start: 6, end: 35 }],
				redactions: vec![],
			}),
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: "Hello sk-abcdefghijklmnopqrstuvwxyz!".to_string(),
		})
		.expect("Expected valid write policy transformation.");
		let expected_audit = elf_domain::writegate::WritePolicyAudit {
			exclusions: vec![WriteSpan { start: 6, end: 35 }],
			..Default::default()
		};

		assert_eq!(validated.content, "Hello !".to_string());
		assert_eq!(validated.write_policy_audit.unwrap_or_default(), expected_audit);
	}

	#[test]
	fn validate_docs_put_rejects_secret_after_write_policy() {
		let err = docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: Some(DocType::Knowledge.as_str().to_string()),
			title: None,
			write_policy: Some(WritePolicy { exclusions: vec![], redactions: vec![] }),
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: "Hello sk-abcdefghijklmnopqrstuvwxyz!".to_string(),
		})
		.expect_err("Expected secret-bearing content to be rejected.");

		match err {
			Error::InvalidRequest { message } => assert!(message.contains("contains secrets")),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn validate_docs_put_allows_doc_source_ref_v1_and_rejects_free_text() {
		docs::validate_docs_put(&DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("English title".to_string()),
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"notes": "English only."
			}),
			content: "English content.".to_string(),
		})
		.expect("Expected doc_source_ref/v1 source_ref to be accepted.");

		let err = docs::validate_docs_put(&DocsPutRequest {
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"notes": "\u{4f60}\u{597d}\u{4e16}\u{754c}"
			}),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("English title".to_string()),
			write_policy: None,
			content: "English content.".to_string(),
		})
		.expect_err("Expected non-English free-text in source_ref.");

		match err {
			Error::NonEnglishInput { field } => assert_eq!(field, "$.source_ref[\"notes\"]"),
			other => panic!("Unexpected error: {other:?}"),
		}

		let err = docs::validate_docs_put(&DocsPutRequest {
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"ref": "\u{4f60}\u{597d}\u{4e16}\u{754c}"
			}),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("English title".to_string()),
			write_policy: None,
			content: "English content.".to_string(),
		})
		.expect_err("Expected identifier lane with non-Latin text to be rejected.");

		match err {
			Error::NonEnglishInput { field } => assert_eq!(field, "$.source_ref[\"ref\"]"),
			other => panic!("Unexpected error: {other:?}"),
		}
	}

	#[test]
	fn split_tokens_by_offsets_preserves_original_substring_offsets() {
		let tokenizer = test_tokenizer();
		let chunks =
			super::split_tokens_by_offsets("alpha bravo charlie delta", 2, 1, 10, &tokenizer)
				.expect("Expected token chunking to succeed.");

		assert_eq!(chunks.len(), 3);
		assert_eq!(chunks[0].start_offset, 0);
		assert_eq!(chunks[0].end_offset, 11);
		assert_eq!(chunks[1].start_offset, 6);
		assert_eq!(chunks[1].end_offset, 19);
		assert_eq!(chunks[2].start_offset, 12);
		assert_eq!(chunks[2].end_offset, 25);

		for chunk in &chunks {
			assert_eq!(
				chunk.text,
				"alpha bravo charlie delta"[chunk.start_offset..chunk.end_offset]
			);
		}
	}
}
