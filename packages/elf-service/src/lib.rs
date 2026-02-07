pub mod add_event;
pub mod add_note;
pub mod admin;
pub mod delete;
pub mod list;
pub mod notes;
pub mod progressive_search;
pub mod search;
pub mod time_serde;
pub mod update;

use std::{future::Future, pin::Pin, sync::Arc};

use serde_json::Value;
use uuid::Uuid;

pub use add_event::{AddEventRequest, AddEventResponse, AddEventResult, EventMessage};
pub use add_note::{AddNoteInput, AddNoteRequest, AddNoteResponse, AddNoteResult};
pub use admin::RebuildReport;
pub use delete::{DeleteRequest, DeleteResponse};
use elf_config::{Config, EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_providers::{embedding, extractor, rerank};
use elf_storage::{db::Db, models::MemoryNote, qdrant::QdrantStore};
pub use list::{ListItem, ListRequest, ListResponse};
pub use notes::{NoteFetchRequest, NoteFetchResponse};
pub use progressive_search::{
	SearchDetailsError, SearchDetailsRequest, SearchDetailsResponse, SearchDetailsResult,
	SearchIndexItem, SearchIndexResponse, SearchSessionGetRequest, SearchTimelineGroup,
	SearchTimelineRequest, SearchTimelineResponse,
};
pub use search::{
	BlendRankingOverride, BlendSegmentOverride, RankingRequestOverride, SearchExplain,
	SearchExplainItem, SearchExplainRequest, SearchExplainResponse, SearchItem, SearchRequest,
	SearchResponse, SearchTrace, TraceGetRequest, TraceGetResponse,
};
pub use update::{UpdateRequest, UpdateResponse};

pub type ServiceResult<T> = Result<T, ServiceError>;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub const REJECT_EVIDENCE_MISMATCH: &str = "REJECT_EVIDENCE_MISMATCH";

pub trait EmbeddingProvider
where
	Self: Send + Sync,
{
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, color_eyre::Result<Vec<Vec<f32>>>>;
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
	) -> BoxFuture<'a, color_eyre::Result<Vec<f32>>>;
}

pub trait ExtractorProvider
where
	Self: Send + Sync,
{
	fn extract<'a>(
		&'a self,
		cfg: &'a LlmProviderConfig,
		messages: &'a [Value],
	) -> BoxFuture<'a, color_eyre::Result<Value>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NoteOp {
	Add,
	Update,
	None,
	Delete,
	Rejected,
}

#[derive(Debug)]
pub enum ServiceError {
	NonEnglishInput { field: String },
	InvalidRequest { message: String },
	ScopeDenied { message: String },
	Provider { message: String },
	Storage { message: String },
	Qdrant { message: String },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum UpdateDecision {
	Add { note_id: Uuid },
	Update { note_id: Uuid },
	None { note_id: Uuid },
}

#[derive(Clone)]
pub struct Providers {
	pub embedding: Arc<dyn EmbeddingProvider>,
	pub rerank: Arc<dyn RerankProvider>,
	pub extractor: Arc<dyn ExtractorProvider>,
}

pub struct ElfService {
	pub cfg: Config,
	pub db: Db,
	pub qdrant: QdrantStore,
	pub providers: Providers,
}

pub(crate) struct ResolveUpdateArgs<'a> {
	pub(crate) cfg: &'a Config,
	pub(crate) providers: &'a Providers,
	pub(crate) tenant_id: &'a str,
	pub(crate) project_id: &'a str,
	pub(crate) agent_id: &'a str,
	pub(crate) scope: &'a str,
	pub(crate) note_type: &'a str,
	pub(crate) key: Option<&'a str>,
	pub(crate) text: &'a str,
	pub(crate) now: time::OffsetDateTime,
}

pub(crate) struct InsertVersionArgs<'a> {
	pub(crate) note_id: Uuid,
	pub(crate) op: &'a str,
	pub(crate) prev_snapshot: Option<Value>,
	pub(crate) new_snapshot: Option<Value>,
	pub(crate) reason: &'a str,
	pub(crate) actor: &'a str,
	pub(crate) ts: time::OffsetDateTime,
}

struct DefaultProviders;

impl std::fmt::Display for ServiceError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NonEnglishInput { field } => {
				write!(f, "Non-English input detected at {field}.")
			},
			Self::InvalidRequest { message } => write!(f, "Invalid request: {message}"),
			Self::ScopeDenied { message } => write!(f, "Scope denied: {message}"),
			Self::Provider { message } => write!(f, "Provider error: {message}"),
			Self::Storage { message } => write!(f, "Storage error: {message}"),
			Self::Qdrant { message } => write!(f, "Qdrant error: {message}"),
		}
	}
}

impl std::error::Error for ServiceError {}

impl From<sqlx::Error> for ServiceError {
	fn from(err: sqlx::Error) -> Self {
		Self::Storage { message: err.to_string() }
	}
}

impl From<color_eyre::Report> for ServiceError {
	fn from(err: color_eyre::Report) -> Self {
		Self::Provider { message: err.to_string() }
	}
}

impl EmbeddingProvider for DefaultProviders {
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, color_eyre::Result<Vec<Vec<f32>>>> {
		Box::pin(embedding::embed(cfg, texts))
	}
}

impl RerankProvider for DefaultProviders {
	fn rerank<'a>(
		&'a self,
		cfg: &'a ProviderConfig,
		query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, color_eyre::Result<Vec<f32>>> {
		Box::pin(rerank::rerank(cfg, query, docs))
	}
}

impl ExtractorProvider for DefaultProviders {
	fn extract<'a>(
		&'a self,
		cfg: &'a LlmProviderConfig,
		messages: &'a [Value],
	) -> BoxFuture<'a, color_eyre::Result<Value>> {
		Box::pin(extractor::extract(cfg, messages))
	}
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

impl ElfService {
	pub fn new(cfg: Config, db: Db, qdrant: QdrantStore) -> Self {
		Self { cfg, db, qdrant, providers: Providers::default() }
	}

	pub fn with_providers(cfg: Config, db: Db, qdrant: QdrantStore, providers: Providers) -> Self {
		Self { cfg, db, qdrant, providers }
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

pub(crate) fn writegate_reason_code(code: elf_domain::writegate::RejectCode) -> &'static str {
	use elf_domain::writegate::RejectCode;
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

pub(crate) fn parse_pg_vector(text: &str) -> Result<Vec<f32>, ServiceError> {
	let trimmed = text.trim();
	let without_brackets =
		trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')).ok_or_else(|| {
			ServiceError::InvalidRequest { message: "Vector text is not bracketed.".to_string() }
		})?;

	if without_brackets.trim().is_empty() {
		return Ok(Vec::new());
	}

	let mut vec = Vec::new();

	for part in without_brackets.split(',') {
		let value: f32 = part.trim().parse().map_err(|_| ServiceError::InvalidRequest {
			message: "Vector text contains a non-numeric value.".to_string(),
		})?;
		vec.push(value);
	}

	Ok(vec)
}

pub(crate) async fn resolve_update(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	args: ResolveUpdateArgs<'_>,
) -> ServiceResult<UpdateDecision> {
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

	if let Some(key) = key.filter(|value| !value.trim().is_empty())
		&& let Some(note_id) = sqlx::query_scalar!(
			"\
SELECT note_id
FROM memory_notes
WHERE tenant_id = $1
	AND project_id = $2
	AND agent_id = $3
	AND scope = $4
	AND type = $5
	AND key = $6
	AND status = 'active'
	AND (expires_at IS NULL OR expires_at > $7)
LIMIT 1",
			tenant_id,
			project_id,
			agent_id,
			scope,
			note_type,
			key,
			now,
		)
		.fetch_optional(&mut **tx)
		.await?
	{
		return Ok(UpdateDecision::Update { note_id });
	}

	let existing_ids: Vec<Uuid> = sqlx::query_scalar!(
		"\
SELECT note_id
FROM memory_notes
WHERE tenant_id = $1
	AND project_id = $2
	AND agent_id = $3
	AND scope = $4
	AND type = $5
	AND status = 'active'
	AND (expires_at IS NULL OR expires_at > $6)",
		tenant_id,
		project_id,
		agent_id,
		scope,
		note_type,
		now,
	)
	.fetch_all(&mut **tx)
	.await?;

	if existing_ids.is_empty() {
		return Ok(UpdateDecision::Add { note_id: Uuid::new_v4() });
	}

	let embeddings =
		providers.embedding.embed(&cfg.providers.embedding, &[text.to_string()]).await?;
	let Some(vec) = embeddings.into_iter().next() else {
		return Err(ServiceError::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		});
	};

	if vec.len() != cfg.storage.qdrant.vector_dim as usize {
		return Err(ServiceError::Provider {
			message: "Embedding vector dimension mismatch.".to_string(),
		});
	}

	let vec_text = vector_to_pg(&vec);
	let embed_version = embedding_version(cfg);
	let rows = sqlx::query!(
		"\
	SELECT
		note_id AS \"note_id!\",
		(1 - (vec <=> $1::text::vector))::real AS \"similarity!\"
	FROM note_embeddings
	WHERE note_id = ANY($2) AND embedding_version = $3",
		vec_text.as_str(),
		existing_ids.as_slice(),
		embed_version.as_str(),
	)
	.fetch_all(&mut **tx)
	.await?;

	let mut best: Option<(Uuid, f32)> = None;

	for row in rows {
		if best.map(|(_, score)| row.similarity > score).unwrap_or(true) {
			best = Some((row.note_id, row.similarity));
		}
	}

	let Some((best_id, best_score)) = best else {
		return Ok(UpdateDecision::Add { note_id: Uuid::new_v4() });
	};

	if best_score >= cfg.memory.dup_sim_threshold {
		return Ok(UpdateDecision::None { note_id: best_id });
	}
	if best_score >= cfg.memory.update_sim_threshold {
		return Ok(UpdateDecision::Update { note_id: best_id });
	}

	Ok(UpdateDecision::Add { note_id: Uuid::new_v4() })
}

pub(crate) async fn insert_version(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	args: InsertVersionArgs<'_>,
) -> ServiceResult<()> {
	let InsertVersionArgs { note_id, op, prev_snapshot, new_snapshot, reason, actor, ts } = args;

	sqlx::query!(
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
		Uuid::new_v4(),
		note_id,
		op,
		prev_snapshot,
		new_snapshot,
		reason,
		actor,
		ts,
	)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

pub(crate) async fn enqueue_outbox_tx(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	note_id: Uuid,
	op: &str,
	embedding_version: &str,
	now: time::OffsetDateTime,
) -> ServiceResult<()> {
	sqlx::query!(
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
		Uuid::new_v4(),
		note_id,
		op,
		embedding_version,
		now,
		now,
		now,
	)
	.execute(&mut **tx)
	.await?;

	Ok(())
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
