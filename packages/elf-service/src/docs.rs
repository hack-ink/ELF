use std::collections::{HashMap, HashSet};

use qdrant_client::{
	Qdrant,
	qdrant::{
		Condition, DatetimeRange, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query,
		QueryPointsBuilder, ScoredPoint, Timestamp,
	},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{FromRow, PgExecutor, PgPool};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokenizers::Tokenizer;
use uuid::Uuid;

use crate::{ElfService, Error, Result, access::SharedSpaceGrantKey};
use elf_config::Config;
use elf_domain::{
	english_gate,
	writegate::{WritePolicy, WritePolicyAudit},
};
use elf_storage::{
	doc_outbox,
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
const DOC_STATUSES: [&str; 2] = ["active", "deleted"];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
	Knowledge,
	Chat,
	Search,
	Dev,
}
impl DocType {
	pub fn as_str(self) -> &'static str {
		match self {
			DocType::Knowledge => "knowledge",
			DocType::Chat => "chat",
			DocType::Search => "search",
			DocType::Dev => "dev",
		}
	}

	pub fn parse(raw_doc_type: &str) -> Result<Self> {
		match raw_doc_type {
			"knowledge" => Ok(DocType::Knowledge),
			"chat" => Ok(DocType::Chat),
			"search" => Ok(DocType::Search),
			"dev" => Ok(DocType::Dev),
			_ => Err(Error::InvalidRequest {
				message: "doc_type must be one of: knowledge, chat, search, dev.".to_string(),
			}),
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
pub struct DocsPutRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub doc_type: Option<String>,
	pub title: Option<String>,
	pub write_policy: Option<WritePolicy>,
	#[serde(default)]
	pub source_ref: Value,
	pub content: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsPutResponse {
	pub doc_id: Uuid,
	pub chunk_count: u32,
	pub content_bytes: u32,
	pub content_hash: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub write_policy_audit: Option<WritePolicyAudit>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DocsGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub doc_id: Uuid,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsGetResponse {
	pub doc_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub doc_type: String,
	pub status: String,
	pub title: Option<String>,
	pub source_ref: Value,
	pub content_bytes: u32,
	pub content_hash: String,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DocsSearchL0Request {
	pub tenant_id: String,
	pub project_id: String,
	pub caller_agent_id: String,
	pub read_profile: String,
	pub query: String,
	pub scope: Option<String>,
	pub status: Option<String>,
	pub doc_type: Option<String>,
	pub sparse_mode: Option<String>,
	pub domain: Option<String>,
	pub repo: Option<String>,
	pub agent_id: Option<String>,
	pub thread_id: Option<String>,
	pub updated_after: Option<String>,
	pub updated_before: Option<String>,
	pub ts_gte: Option<String>,
	pub ts_lte: Option<String>,
	pub top_k: Option<u32>,
	pub candidate_k: Option<u32>,
	pub explain: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Item {
	pub doc_id: Uuid,
	pub chunk_id: Uuid,
	pub pointer: DocsSearchL0ItemPointer,
	pub score: f32,
	pub snippet: String,
	pub scope: String,
	pub doc_type: String,
	pub project_id: String,
	pub agent_id: String,
	pub updated_at: OffsetDateTime,
	pub content_hash: String,
	pub chunk_hash: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Response {
	pub trace_id: Uuid,
	pub items: Vec<DocsSearchL0Item>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trajectory: Option<DocRetrievalTrajectory>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemPointer {
	pub schema: String,
	pub resolver: String,
	#[serde(rename = "ref")]
	pub reference: DocsSearchL0ItemReference,
	pub state: DocsSearchL0ItemState,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemReference {
	pub doc_id: Uuid,
	pub chunk_id: Uuid,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemState {
	pub content_hash: String,
	pub chunk_hash: String,
	#[serde(with = "crate::time_serde")]
	pub doc_updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocRetrievalTrajectory {
	pub schema: String,
	pub stages: Vec<DocRetrievalTrajectoryStage>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocRetrievalTrajectoryStage {
	pub stage_order: u32,
	pub stage_name: String,
	pub stats: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextQuoteSelector {
	pub exact: String,
	pub prefix: Option<String>,
	pub suffix: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextPositionSelector {
	pub start: usize,
	pub end: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DocsExcerptsGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub doc_id: Uuid,
	pub level: String, // "L0" | "L1" | "L2"
	pub chunk_id: Option<Uuid>,
	pub quote: Option<TextQuoteSelector>,
	pub position: Option<TextPositionSelector>,
	pub explain: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptVerification {
	pub verified: bool,
	pub verification_errors: Vec<String>,
	pub content_hash: String,
	pub excerpt_hash: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptResponse {
	pub trace_id: Uuid,
	pub doc_id: Uuid,
	pub excerpt: String,
	pub start_offset: usize,
	pub end_offset: usize,
	pub locator: DocsExcerptLocator,
	pub verification: DocsExcerptVerification,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trajectory: Option<DocRetrievalTrajectory>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptLocator {
	pub selector_kind: String,
	pub match_start_offset: usize,
	pub match_end_offset: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub chunk_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub quote: Option<TextQuoteSelector>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub position: Option<TextPositionSelector>,
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
	pub async fn docs_put(&self, req: DocsPutRequest) -> Result<DocsPutResponse> {
		let ValidatedDocsPut { doc_type, content, write_policy_audit } = validate_docs_put(&req)?;
		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let DocsPutRequest { tenant_id, project_id, agent_id, scope, title, source_ref, .. } = req;
		let chunking_profile = resolve_doc_chunking_profile(doc_type);
		let tokenizer = load_tokenizer(&self.cfg)?;
		let effective_project_id = if scope.trim() == "org_shared" {
			crate::access::ORG_PROJECT_ID
		} else {
			project_id.as_str()
		};
		let content_bytes = content.len();
		let content_hash = blake3::hash(content.as_bytes());
		let doc_id = Uuid::new_v4();
		let chunks = split_tokens_by_offsets(
			content.as_str(),
			chunking_profile.max_tokens,
			chunking_profile.overlap_tokens,
			chunking_profile.max_chunks,
			&tokenizer,
		)?;
		let doc_row = DocDocument {
			doc_id,
			tenant_id: tenant_id.clone(),
			project_id: effective_project_id.to_string(),
			agent_id: agent_id.clone(),
			scope: scope.clone(),
			doc_type: doc_type.as_str().to_string(),
			status: "active".to_string(),
			title,
			source_ref: elf_storage::docs::normalize_source_ref(Some(source_ref)),
			content,
			content_bytes: content_bytes as i32,
			content_hash: content_hash.to_hex().to_string(),
			created_at: now,
			updated_at: now,
		};
		let mut tx = self.db.pool.begin().await?;

		elf_storage::docs::insert_doc_document(&mut *tx, &doc_row).await?;

		for (chunk_index, chunk) in chunks.iter().enumerate() {
			let chunk_hash = blake3::hash(chunk.text.as_bytes());
			let chunk_row = DocChunk {
				chunk_id: chunk.chunk_id,
				doc_id,
				chunk_index: chunk_index as i32,
				start_offset: chunk.start_offset as i32,
				end_offset: chunk.end_offset as i32,
				chunk_text: chunk.text.clone(),
				chunk_hash: chunk_hash.to_hex().to_string(),
				created_at: now,
			};

			elf_storage::docs::insert_doc_chunk(&mut *tx, &chunk_row).await?;
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
			crate::access::ensure_active_project_scope_grant(
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
			chunk_count: chunks.len() as u32,
			content_bytes: content_bytes as u32,
			content_hash: content_hash.to_hex().to_string(),
			write_policy_audit,
		})
	}

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

		let allowed_scopes = crate::search::resolve_read_profile_scopes(&self.cfg, read_profile)?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let row: Option<DocDocument> = sqlx::query_as::<_, DocDocument>(
			"\
SELECT
\tdoc_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tdoc_type,
\tstatus,
\ttitle,
\tCOALESCE(source_ref, '{}'::jsonb) AS source_ref,
\tcontent,
\tcontent_bytes,
\tcontent_hash,
\tcreated_at,
\tupdated_at
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
		.bind(crate::access::ORG_PROJECT_ID)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::NotFound { message: "Doc not found.".to_string() });
		};
		let shared_grants = if row.scope == "agent_private" {
			HashSet::new()
		} else {
			crate::access::load_shared_read_grants_with_org_shared(
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
			crate::search::resolve_read_profile_scopes(&self.cfg, req.read_profile.as_str())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = crate::access::load_shared_read_grants_with_org_shared(
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
			.embed(&self.cfg.providers.embedding, std::slice::from_ref(&req.query))
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
) -> DocsExcerptLocator {
	DocsExcerptLocator {
		selector_kind: selector_kind.as_str().to_string(),
		match_start_offset,
		match_end_offset,
		chunk_id: req.chunk_id,
		quote: req.quote.clone(),
		position: req.position.clone(),
	}
}

fn build_docs_l0_pointer(row: &DocSearchRow, chunk_id: Uuid) -> DocsSearchL0ItemPointer {
	DocsSearchL0ItemPointer {
		schema: DOC_SOURCE_REF_SCHEMA_V1.to_string(),
		resolver: DOC_SOURCE_REF_RESOLVER_V1.to_string(),
		reference: DocsSearchL0ItemReference { doc_id: row.doc_id, chunk_id },
		state: DocsSearchL0ItemState {
			content_hash: row.content_hash.clone(),
			chunk_hash: row.chunk_hash.clone(),
			doc_updated_at: row.updated_at,
		},
	}
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

	let write_policy =
		elf_domain::writegate::apply_write_policy(req.content.as_str(), req.write_policy.as_ref())
			.map_err(|err| Error::InvalidRequest {
				message: format!("write_policy is invalid: {err:?}"),
			})?;
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
	if elf_domain::writegate::contains_secrets(content.as_str()) {
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

	Tokenizer::from_pretrained(tokenizer_repo, None).map_err(|err| Error::InvalidRequest {
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
			Condition::matches("project_id", crate::access::ORG_PROJECT_ID.to_string()),
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
	use qdrant_client::qdrant::point_id::PointIdOptions;

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
	let allowed_scopes = crate::search::resolve_read_profile_scopes(cfg, read_profile)?;
	let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
	let shared_grants = crate::access::load_shared_read_grants_with_org_shared(
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
\tdoc_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tdoc_type,
\tstatus,
\ttitle,
\tCOALESCE(source_ref, '{}'::jsonb) AS source_ref,
\tcontent,
\tcontent_bytes,
\tcontent_hash,
\tcreated_at,
\tupdated_at
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
	.bind(crate::access::ORG_PROJECT_ID)
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
		let chunk = elf_storage::docs::get_doc_chunk(pool, chunk_id).await?;
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
			.query(Query::new_nearest(qdrant_client::qdrant::Document::new(
				query_text.to_string(),
				BM25_MODEL,
			)))
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
	.bind(crate::access::ORG_PROJECT_ID)
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
	use crate::docs::{
		DocType, DocsPutRequest, DocsSearchL0Filters, DocsSearchL0Request, DocsSparseMode, Error,
		resolve_doc_chunking_profile, validate_docs_put, validate_docs_search_l0,
	};
	use ahash::AHashMap;
	use elf_domain::writegate::{WritePolicy, WriteSpan};
	use qdrant_client::qdrant::{
		DatetimeRange, Filter, condition::ConditionOneOf, r#match::MatchValue,
	};
	use time::{OffsetDateTime, format_description::well_known::Rfc3339};
	use tokenizers::{
		Tokenizer, models::wordlevel::WordLevel, pre_tokenizers::whitespace::Whitespace,
	};

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
		let err = validate_docs_search_l0(&DocsSearchL0Request {
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

		validate_docs_search_l0(&DocsSearchL0Request {
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
		let err = validate_docs_put(&DocsPutRequest {
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
		let small = resolve_doc_chunking_profile(DocType::Chat);

		assert_eq!(small.max_tokens, 1_024);
		assert_eq!(small.overlap_tokens, 128);

		let default = resolve_doc_chunking_profile(DocType::Knowledge);

		assert_eq!(default.max_tokens, 2_048);
		assert_eq!(default.overlap_tokens, 256);
	}

	#[test]
	fn validate_docs_search_l0_defaults_status_and_filters_dates() {
		let filters = validate_docs_search_l0(&test_request_with_query("hello world"))
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
		let err = validate_docs_search_l0(&bad_dates)
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
		let err = validate_docs_search_l0(&DocsSearchL0Request {
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
		let err = validate_docs_search_l0(&DocsSearchL0Request {
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
		let err = validate_docs_search_l0(&DocsSearchL0Request {
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
		let err = validate_docs_search_l0(&DocsSearchL0Request {
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
		let err = validate_docs_search_l0(&DocsSearchL0Request {
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
		let err = validate_docs_search_l0(&DocsSearchL0Request {
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
		let filters =
			validate_docs_search_l0(&test_request_with_query("status")).expect("valid request");

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
		let err = validate_docs_put(&DocsPutRequest {
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
		let err = validate_docs_put(&DocsPutRequest {
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
		let err = validate_docs_put(&DocsPutRequest {
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
		let err = validate_docs_put(&DocsPutRequest {
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
		let err = validate_docs_put(&DocsPutRequest {
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
		let err = validate_docs_put(&DocsPutRequest {
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
		let err = validate_docs_put(&DocsPutRequest {
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
		let resolved_doc_type = validate_docs_put(&DocsPutRequest {
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
	fn validate_docs_put_applies_write_policy_and_includes_audit() {
		let validated = validate_docs_put(&DocsPutRequest {
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
		let mut expected_audit = elf_domain::writegate::WritePolicyAudit::default();

		expected_audit.exclusions = vec![WriteSpan { start: 6, end: 35 }];

		assert_eq!(validated.content, "Hello !".to_string());
		assert_eq!(validated.write_policy_audit.unwrap_or_default(), expected_audit);
	}

	#[test]
	fn validate_docs_put_rejects_secret_after_write_policy() {
		let err = validate_docs_put(&DocsPutRequest {
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
		validate_docs_put(&DocsPutRequest {
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

		let err = validate_docs_put(&DocsPutRequest {
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

		let err = validate_docs_put(&DocsPutRequest {
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
