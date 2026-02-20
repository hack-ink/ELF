pub mod add_event;
pub mod add_note;
pub mod admin;
pub mod admin_graph_predicates;
pub mod delete;
pub mod graph;
pub mod list;
pub mod notes;
pub mod progressive_search;
pub mod search;
pub mod structured_fields;
pub mod time_serde;
pub mod update;

mod error;
mod graph_ingestion;
mod ingest_audit;
mod ranking_explain_v2;

pub use self::{
	add_event::{AddEventRequest, AddEventResponse, AddEventResult, EventMessage},
	add_note::{AddNoteInput, AddNoteRequest, AddNoteResponse, AddNoteResult},
	admin::RebuildReport,
	admin_graph_predicates::{
		AdminGraphPredicateAliasAddRequest, AdminGraphPredicateAliasResponse,
		AdminGraphPredicateAliasesListRequest, AdminGraphPredicateAliasesResponse,
		AdminGraphPredicatePatchRequest, AdminGraphPredicateResponse,
		AdminGraphPredicatesListRequest, AdminGraphPredicatesListResponse,
	},
	delete::{DeleteRequest, DeleteResponse},
	error::{Error, Result},
	list::{ListItem, ListRequest, ListResponse},
	notes::{NoteFetchRequest, NoteFetchResponse},
	progressive_search::{
		SearchDetailsError, SearchDetailsRequest, SearchDetailsResponse, SearchDetailsResult,
		SearchIndexItem, SearchIndexPlannedResponse, SearchIndexResponse, SearchSessionGetRequest,
		SearchTimelineGroup, SearchTimelineRequest, SearchTimelineResponse,
	},
	search::{
		BlendRankingOverride, BlendSegmentOverride, PayloadLevel, QueryPlan, QueryPlanBlendSegment,
		QueryPlanBudget, QueryPlanDynamicGate, QueryPlanFusionPolicy, QueryPlanIntent,
		QueryPlanRerankPolicy, QueryPlanRetrievalStage, QueryPlanRewrite, QueryPlanStage,
		RankingRequestOverride, SearchExplain, SearchExplainItem, SearchExplainRequest,
		SearchExplainResponse, SearchExplainTrajectory, SearchExplainTrajectoryStage, SearchItem,
		SearchRawPlannedResponse, SearchRequest, SearchResponse, SearchTrace,
		SearchTrajectoryResponse, SearchTrajectoryStage, SearchTrajectoryStageItem,
		SearchTrajectorySummary, SearchTrajectorySummaryStage, TraceGetRequest, TraceGetResponse,
		TraceTrajectoryGetRequest,
	},
	structured_fields::StructuredFields,
	update::{UpdateRequest, UpdateResponse},
};

use std::{future::Future, pin::Pin, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_config::{Config, EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_domain::writegate::RejectCode;
use elf_providers::{embedding, extractor};
use elf_storage::{db::Db, models::MemoryNote, qdrant::QdrantStore};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub const REJECT_EVIDENCE_MISMATCH: &str = "REJECT_EVIDENCE_MISMATCH";

const RESOLVE_UPDATE_QUERY: &str = "\
WITH key_match AS (
	SELECT note_id
	FROM memory_notes
	WHERE tenant_id = $1
		AND project_id = $2
		AND agent_id = $3
		AND scope = $4
		AND type = $5
		AND $6::text IS NOT NULL
		AND key = $6
		AND status = 'active'
		AND (expires_at IS NULL OR expires_at > $7)
	LIMIT 1
),
existing AS (
	SELECT note_id
	FROM memory_notes
	WHERE tenant_id = $1
		AND project_id = $2
		AND agent_id = $3
		AND scope = $4
		AND type = $5
		AND status = 'active'
		AND (expires_at IS NULL OR expires_at > $7)
),
best AS (
	SELECT
		note_id,
		(1 - (vec <=> $8::text::vector))::real AS similarity
	FROM note_embeddings
	WHERE note_id = ANY(ARRAY(SELECT note_id FROM existing))
		AND embedding_version = $9
	ORDER BY similarity DESC
	LIMIT 1
)
	SELECT
		(SELECT note_id FROM key_match) AS key_note_id,
		(SELECT note_id FROM best) AS best_note_id,
		(SELECT similarity FROM best) AS best_similarity";

pub trait EmbeddingProvider
where
	Self: Send + Sync,
{
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>>;
}

pub trait RerankProvider
where
	Self: Send + Sync,
{
	fn rerank<'a>(
		&'a self,
		cfg: &'a ProviderConfig,
		query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>>;
}

pub trait ExtractorProvider
where
	Self: Send + Sync,
{
	fn extract<'a>(
		&'a self,
		cfg: &'a LlmProviderConfig,
		messages: &'a [Value],
	) -> BoxFuture<'a, Result<Value>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NoteOp {
	Add,
	Update,
	None,
	Delete,
	Rejected,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UpdateDecisionMetadata {
	pub similarity_best: Option<f32>,
	pub key_match: bool,
	pub matched_dup: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum UpdateDecision {
	Add { note_id: Uuid, metadata: UpdateDecisionMetadata },
	Update { note_id: Uuid, metadata: UpdateDecisionMetadata },
	None { note_id: Uuid, metadata: UpdateDecisionMetadata },
}
impl UpdateDecision {
	pub(crate) fn note_id(&self) -> Uuid {
		match self {
			Self::Add { note_id, .. }
			| Self::Update { note_id, .. }
			| Self::None { note_id, .. } => *note_id,
		}
	}

	pub(crate) fn metadata(&self) -> UpdateDecisionMetadata {
		match self {
			Self::Add { metadata, .. }
			| Self::Update { metadata, .. }
			| Self::None { metadata, .. } => *metadata,
		}
	}
}

#[derive(Clone)]
pub struct Providers {
	pub embedding: Arc<dyn EmbeddingProvider>,
	pub rerank: Arc<dyn RerankProvider>,
	pub extractor: Arc<dyn ExtractorProvider>,
}
impl Providers {
	pub fn new(
		embedding: Arc<dyn EmbeddingProvider>,
		rerank: Arc<dyn RerankProvider>,
		extractor: Arc<dyn ExtractorProvider>,
	) -> Self {
		Self { embedding, rerank, extractor }
	}
}

impl Default for Providers {
	fn default() -> Self {
		let provider = Arc::new(DefaultProviders);

		Self { embedding: provider.clone(), rerank: provider.clone(), extractor: provider }
	}
}

pub struct ElfService {
	pub cfg: Config,
	pub db: Db,
	pub qdrant: QdrantStore,
	pub providers: Providers,
}
impl ElfService {
	pub fn new(cfg: Config, db: Db, qdrant: QdrantStore) -> Self {
		Self { cfg, db, qdrant, providers: Providers::default() }
	}

	pub fn with_providers(cfg: Config, db: Db, qdrant: QdrantStore, providers: Providers) -> Self {
		Self { cfg, db, qdrant, providers }
	}
}

struct ResolveUpdateArgs<'a> {
	pub(crate) cfg: &'a Config,
	pub(crate) providers: &'a Providers,
	pub(crate) tenant_id: &'a str,
	pub(crate) project_id: &'a str,
	pub(crate) agent_id: &'a str,
	pub(crate) scope: &'a str,
	pub(crate) note_type: &'a str,
	pub(crate) key: Option<&'a str>,
	pub(crate) text: &'a str,
	pub(crate) now: OffsetDateTime,
}

struct InsertVersionArgs<'a> {
	pub(crate) note_id: Uuid,
	pub(crate) op: &'a str,
	pub(crate) prev_snapshot: Option<Value>,
	pub(crate) new_snapshot: Option<Value>,
	pub(crate) reason: &'a str,
	pub(crate) actor: &'a str,
	pub(crate) ts: OffsetDateTime,
}

struct DefaultProviders;
impl EmbeddingProvider for DefaultProviders {
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		Box::pin(async move {
			embedding::embed(cfg, texts)
				.await
				.map_err(|err| Error::Provider { message: err.to_string() })
		})
	}
}

impl RerankProvider for DefaultProviders {
	fn rerank<'a>(
		&'a self,
		cfg: &'a ProviderConfig,
		query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
		Box::pin(async move {
			elf_providers::rerank::rerank(cfg, query, docs)
				.await
				.map_err(|err| Error::Provider { message: err.to_string() })
		})
	}
}

impl ExtractorProvider for DefaultProviders {
	fn extract<'a>(
		&'a self,
		cfg: &'a LlmProviderConfig,
		messages: &'a [Value],
	) -> BoxFuture<'a, Result<Value>> {
		Box::pin(async move {
			extractor::extract(cfg, messages)
				.await
				.map_err(|err| Error::Provider { message: err.to_string() })
		})
	}
}

pub(crate) fn embedding_version(cfg: &Config) -> String {
	format!(
		"{}:{}:{}",
		cfg.providers.embedding.provider_id,
		cfg.providers.embedding.model,
		cfg.storage.qdrant.vector_dim
	)
}

pub(crate) fn writegate_reason_code(code: RejectCode) -> &'static str {
	match code {
		RejectCode::RejectCjk => "REJECT_CJK",
		RejectCode::RejectTooLong => "REJECT_TOO_LONG",
		RejectCode::RejectSecret => "REJECT_SECRET",
		RejectCode::RejectInvalidType => "REJECT_INVALID_TYPE",
		RejectCode::RejectScopeDenied => "REJECT_SCOPE_DENIED",
		RejectCode::RejectEmpty => "REJECT_EMPTY",
	}
}

pub(crate) fn vector_to_pg(vec: &[f32]) -> String {
	let mut out = String::with_capacity(vec.len() * 8);

	out.push('[');

	for (i, value) in vec.iter().enumerate() {
		if i > 0 {
			out.push(',');
		}

		out.push_str(&value.to_string());
	}

	out.push(']');

	out
}

pub(crate) fn parse_pg_vector(text: &str) -> Result<Vec<f32>> {
	let trimmed = text.trim();
	let without_brackets =
		trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')).ok_or_else(|| {
			Error::InvalidRequest { message: "Vector text is not bracketed.".to_string() }
		})?;

	if without_brackets.trim().is_empty() {
		return Ok(Vec::new());
	}

	let mut vec = Vec::new();

	for part in without_brackets.split(',') {
		let value: f32 = part.trim().parse().map_err(|_| Error::InvalidRequest {
			message: "Vector text contains a non-numeric value.".to_string(),
		})?;

		vec.push(value);
	}

	Ok(vec)
}

pub(crate) fn note_snapshot(note: &MemoryNote) -> Value {
	serde_json::json!({
		"note_id": note.note_id,
		"tenant_id": note.tenant_id,
		"project_id": note.project_id,
		"agent_id": note.agent_id,
		"scope": note.scope,
		"type": note.r#type,
		"key": note.key,
		"text": note.text,
		"importance": note.importance,
		"confidence": note.confidence,
		"status": note.status,
		"created_at": note.created_at,
		"updated_at": note.updated_at,
		"expires_at": note.expires_at,
		"embedding_version": note.embedding_version,
		"source_ref": note.source_ref,
		"hit_count": note.hit_count,
		"last_hit_at": note.last_hit_at,
	})
}

pub(crate) async fn resolve_update<'e, E>(
	executor: E,
	args: ResolveUpdateArgs<'_>,
) -> Result<UpdateDecision>
where
	E: PgExecutor<'e>,
{
	let ResolveUpdateArgs {
		cfg,
		providers,
		tenant_id,
		project_id,
		agent_id,
		scope,
		note_type,
		key,
		text,
		now,
	} = args;
	let embeddings =
		providers.embedding.embed(&cfg.providers.embedding, &[text.to_string()]).await?;
	let Some(vec) = embeddings.into_iter().next() else {
		return Err(Error::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		});
	};

	if vec.len() != cfg.storage.qdrant.vector_dim as usize {
		return Err(Error::Provider {
			message: "Embedding vector dimension mismatch.".to_string(),
		});
	}

	let vec_text = vector_to_pg(&vec);
	let embed_version = embedding_version(cfg);
	let key = key.map(|value| value.trim()).filter(|value| !value.is_empty());
	let row: (Option<Uuid>, Option<Uuid>, Option<f32>) = sqlx::query_as(RESOLVE_UPDATE_QUERY)
		.bind(tenant_id)
		.bind(project_id)
		.bind(agent_id)
		.bind(scope)
		.bind(note_type)
		.bind(key)
		.bind(now)
		.bind(vec_text.as_str())
		.bind(embed_version.as_str())
		.fetch_one(executor)
		.await?;
	let (key_note_id, best_note_id, best_similarity) = row;

	if let Some(note_id) = key_note_id {
		return Ok(UpdateDecision::Update {
			note_id,
			metadata: UpdateDecisionMetadata {
				similarity_best: None,
				key_match: true,
				matched_dup: false,
			},
		});
	}

	let Some(best_id) = best_note_id else {
		return Ok(UpdateDecision::Add {
			note_id: Uuid::new_v4(),
			metadata: UpdateDecisionMetadata {
				similarity_best: None,
				key_match: false,
				matched_dup: false,
			},
		});
	};
	let Some(best_score) = best_similarity else {
		return Ok(UpdateDecision::Add {
			note_id: Uuid::new_v4(),
			metadata: UpdateDecisionMetadata {
				similarity_best: None,
				key_match: false,
				matched_dup: false,
			},
		});
	};

	if best_score >= cfg.memory.dup_sim_threshold {
		return Ok(UpdateDecision::None {
			note_id: best_id,
			metadata: UpdateDecisionMetadata {
				similarity_best: Some(best_score),
				key_match: false,
				matched_dup: true,
			},
		});
	}
	if best_score >= cfg.memory.update_sim_threshold {
		return Ok(UpdateDecision::Update {
			note_id: best_id,
			metadata: UpdateDecisionMetadata {
				similarity_best: Some(best_score),
				key_match: false,
				matched_dup: false,
			},
		});
	}

	Ok(UpdateDecision::Add {
		note_id: Uuid::new_v4(),
		metadata: UpdateDecisionMetadata {
			similarity_best: Some(best_score),
			key_match: false,
			matched_dup: false,
		},
	})
}

pub(crate) async fn insert_version<'e, E>(executor: E, args: InsertVersionArgs<'_>) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let InsertVersionArgs { note_id, op, prev_snapshot, new_snapshot, reason, actor, ts } = args;

	sqlx::query(
		"\
INSERT INTO memory_note_versions (
	version_id,
	note_id,
	op,
	prev_snapshot,
	new_snapshot,
	reason,
	actor,
	ts
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
	)
	.bind(Uuid::new_v4())
	.bind(note_id)
	.bind(op)
	.bind(prev_snapshot)
	.bind(new_snapshot)
	.bind(reason)
	.bind(actor)
	.bind(ts)
	.execute(executor)
	.await?;

	Ok(())
}

pub(crate) async fn enqueue_outbox_tx<'e, E>(
	executor: E,
	note_id: Uuid,
	op: &str,
	embedding_version: &str,
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO indexing_outbox (
	outbox_id,
	note_id,
	op,
	embedding_version,
	status,
	created_at,
	updated_at,
	available_at
)
VALUES ($1,$2,$3,$4,'PENDING',$5,$6,$7)",
	)
	.bind(Uuid::new_v4())
	.bind(note_id)
	.bind(op)
	.bind(embedding_version)
	.bind(now)
	.bind(now)
	.bind(now)
	.execute(executor)
	.await?;

	Ok(())
}
