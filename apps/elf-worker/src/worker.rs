use std::collections::HashMap;

use color_eyre::{Result, eyre::eyre};
use elf_storage::{
	db::Db,
	models::{IndexingOutboxEntry, MemoryNote},
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME, QdrantStore},
};
use qdrant_client::{
	client::Payload,
	qdrant::{DeletePointsBuilder, Document, PointStruct, UpsertPointsBuilder, Value, Vector},
};
use serde::Serialize;
use serde_json::{Value as JsonValue, Value as SerdeValue};
use sqlx::Row;
use time::{Duration, OffsetDateTime};
use tracing::{error, info};

const POLL_INTERVAL_MS: i64 = 500;
const CLAIM_LEASE_SECONDS: i64 = 30;
const BASE_BACKOFF_MS: i64 = 500;
const MAX_BACKOFF_MS: i64 = 30_000;
const TRACE_CLEANUP_INTERVAL_SECONDS: i64 = 900;
const TRACE_OUTBOX_LEASE_SECONDS: i64 = 30;

#[derive(Debug, serde::Deserialize)]
struct TracePayload {
	trace: TraceRecord,
	items: Vec<TraceItemRecord>,
}

#[derive(Debug, serde::Deserialize)]
struct TraceRecord {
	trace_id: uuid::Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	expansion_mode: String,
	expanded_queries: Vec<String>,
	allowed_scopes: Vec<String>,
	candidate_count: u32,
	top_k: u32,
	config_snapshot: SerdeValue,
	trace_version: i32,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

#[derive(Debug, serde::Deserialize)]
struct TraceItemRecord {
	item_id: uuid::Uuid,
	note_id: uuid::Uuid,
	rank: u32,
	retrieval_score: Option<f32>,
	retrieval_rank: Option<u32>,
	rerank_score: f32,
	tie_breaker_score: f32,
	final_score: f32,
	boosts: Vec<TraceBoost>,
	matched_terms: Vec<String>,
	matched_fields: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct TraceBoost {
	name: String,
	score: f32,
}

struct TraceOutboxJob {
	outbox_id: uuid::Uuid,
	trace_id: uuid::Uuid,
	payload: SerdeValue,
	attempts: i32,
}

struct TraceItemInsert {
	item_id: uuid::Uuid,
	note_id: uuid::Uuid,
	rank: i32,
	retrieval_score: Option<f32>,
	retrieval_rank: Option<i32>,
	rerank_score: f32,
	tie_breaker_score: f32,
	final_score: f32,
	boosts: SerdeValue,
	matched_terms: SerdeValue,
	matched_fields: SerdeValue,
}

pub struct WorkerState {
	pub db: Db,
	pub qdrant: QdrantStore,
	pub embedding: elf_config::EmbeddingProviderConfig,
}

pub async fn run_worker(state: WorkerState) -> Result<()> {
	let mut last_trace_cleanup = OffsetDateTime::now_utc();
	loop {
		if let Err(err) = process_indexing_outbox_once(&state).await {
			error!(error = %err, "Indexing outbox processing failed.");
		}
		if let Err(err) = process_trace_outbox_once(&state).await {
			error!(error = %err, "Search trace outbox processing failed.");
		}
		let now = OffsetDateTime::now_utc();
		if now - last_trace_cleanup >= Duration::seconds(TRACE_CLEANUP_INTERVAL_SECONDS) {
			if let Err(err) = purge_expired_traces(&state.db, now).await {
				error!(error = %err, "Search trace cleanup failed.");
			} else {
				last_trace_cleanup = now;
			}
		}
		tokio::time::sleep(to_std_duration(Duration::milliseconds(POLL_INTERVAL_MS))).await;
	}
}

async fn process_indexing_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job = fetch_next_job(&state.db, now).await?;
	let Some(job) = job else {
		return Ok(());
	};

	let result = match job.op.as_str() {
		"UPSERT" => handle_upsert(state, &job).await,
		"DELETE" => handle_delete(state, &job).await,
		other => Err(eyre!("Unsupported outbox op: {other}.")),
	};

	match result {
		Ok(()) => {
			mark_done(&state.db, job.outbox_id).await?;
		},
		Err(err) => {
			mark_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
			error!(error = %err, outbox_id = %job.outbox_id, "Outbox job failed.");
		},
	}

	Ok(())
}

async fn process_trace_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job = fetch_next_trace_job(&state.db, now).await?;
	let Some(job) = job else {
		return Ok(());
	};

	let result = handle_trace_job(&state.db, &job).await;
	match result {
		Ok(()) => {
			mark_trace_done(&state.db, job.outbox_id).await?;
		},
		Err(err) => {
			mark_trace_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
			error!(error = %err, trace_id = %job.trace_id, "Search trace outbox job failed.");
		},
	}

	Ok(())
}

// TODO: Add outbox fetch/update helpers in elf_storage::outbox and use them here.
async fn fetch_next_job(db: &Db, now: OffsetDateTime) -> Result<Option<IndexingOutboxEntry>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query(
        "SELECT outbox_id, note_id, op, embedding_version, status, attempts, last_error, available_at, created_at, updated_at \
         FROM indexing_outbox \
         WHERE status IN ('PENDING','FAILED') AND available_at <= $1 \
         ORDER BY available_at ASC \
         LIMIT 1 \
         FOR UPDATE SKIP LOCKED",
    )
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

	let job = if let Some(row) = row {
		let outbox_id = row.try_get("outbox_id")?;
		let note_id = row.try_get("note_id")?;
		let op = row.try_get("op")?;
		let embedding_version = row.try_get("embedding_version")?;
		let status = row.try_get("status")?;
		let attempts = row.try_get("attempts")?;
		let last_error = row.try_get("last_error")?;
		let available_at = row.try_get("available_at")?;
		let created_at = row.try_get("created_at")?;
		let updated_at = row.try_get("updated_at")?;

		let lease_until = now + Duration::seconds(CLAIM_LEASE_SECONDS);
		sqlx::query(
			"UPDATE indexing_outbox SET available_at = $1, updated_at = $2 WHERE outbox_id = $3",
		)
		.bind(lease_until)
		.bind(now)
		.bind(outbox_id)
		.execute(&mut *tx)
		.await?;

		Some(IndexingOutboxEntry {
			outbox_id,
			note_id,
			op,
			embedding_version,
			status,
			attempts,
			last_error,
			available_at,
			created_at,
			updated_at,
		})
	} else {
		None
	};

	tx.commit().await?;
	Ok(job)
}

async fn fetch_next_trace_job(db: &Db, now: OffsetDateTime) -> Result<Option<TraceOutboxJob>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query(
		"SELECT outbox_id, trace_id, payload, attempts \
         FROM search_trace_outbox \
         WHERE status IN ('PENDING','FAILED') AND available_at <= $1 \
         ORDER BY available_at ASC \
         LIMIT 1 \
         FOR UPDATE SKIP LOCKED",
	)
	.bind(now)
	.fetch_optional(&mut *tx)
	.await?;

	let job = if let Some(row) = row {
		let outbox_id = row.try_get("outbox_id")?;
		let trace_id = row.try_get("trace_id")?;
		let payload = row.try_get("payload")?;
		let attempts = row.try_get("attempts")?;

		let lease_until = now + Duration::seconds(TRACE_OUTBOX_LEASE_SECONDS);
		sqlx::query(
			"UPDATE search_trace_outbox SET available_at = $1, updated_at = $2 WHERE outbox_id = $3",
		)
		.bind(lease_until)
		.bind(now)
		.bind(outbox_id)
		.execute(&mut *tx)
		.await?;

		Some(TraceOutboxJob { outbox_id, trace_id, payload, attempts })
	} else {
		None
	};

	tx.commit().await?;
	Ok(job)
}

async fn handle_upsert(state: &WorkerState, job: &IndexingOutboxEntry) -> Result<()> {
	let note = fetch_note(&state.db, job.note_id).await?;
	let Some(note) = note else {
		info!(note_id = %job.note_id, "Note missing for outbox job. Marking done.");
		return Ok(());
	};

	let now = OffsetDateTime::now_utc();
	if !note_is_active(&note, now) {
		info!(note_id = %job.note_id, "Note inactive or expired. Skipping index.");
		return Ok(());
	}

	let embedding = ensure_embedding(state, &note, &job.embedding_version).await?;
	upsert_qdrant(state, &note, &embedding).await?;
	Ok(())
}

async fn handle_delete(state: &WorkerState, job: &IndexingOutboxEntry) -> Result<()> {
	let point_id = job.note_id.to_string();
	let delete =
		DeletePointsBuilder::new(state.qdrant.collection.clone()).points([point_id]).wait(true);
	match state.qdrant.client.delete_points(delete).await {
		Ok(_) => {},
		Err(err) =>
			if is_not_found_error(&err) {
				info!(outbox_id = %job.outbox_id, "Qdrant point missing during delete.");
			} else {
				return Err(eyre!(err.to_string()));
			},
	}
	Ok(())
}

async fn handle_trace_job(db: &Db, job: &TraceOutboxJob) -> Result<()> {
	let payload: TracePayload = serde_json::from_value(job.payload.clone())?;
	let trace = payload.trace;
	let trace_id = trace.trace_id;
	let mut tx = db.pool.begin().await?;

	sqlx::query(
		"INSERT INTO search_traces \
         (trace_id, tenant_id, project_id, agent_id, read_profile, query, expansion_mode, \
          expanded_queries, allowed_scopes, candidate_count, top_k, config_snapshot, \
          trace_version, created_at, expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) \
         ON CONFLICT (trace_id) DO NOTHING",
	)
	.bind(trace_id)
	.bind(&trace.tenant_id)
	.bind(&trace.project_id)
	.bind(&trace.agent_id)
	.bind(&trace.read_profile)
	.bind(&trace.query)
	.bind(&trace.expansion_mode)
	.bind(encode_json(&trace.expanded_queries, "expanded_queries")?)
	.bind(encode_json(&trace.allowed_scopes, "allowed_scopes")?)
	.bind(trace.candidate_count as i32)
	.bind(trace.top_k as i32)
	.bind(trace.config_snapshot.clone())
	.bind(trace.trace_version)
	.bind(trace.created_at)
	.bind(trace.expires_at)
	.execute(&mut *tx)
	.await?;

	if !payload.items.is_empty() {
		let mut inserts = Vec::with_capacity(payload.items.len());
		for item in payload.items {
			inserts.push(TraceItemInsert {
				item_id: item.item_id,
				note_id: item.note_id,
				rank: item.rank as i32,
				retrieval_score: item.retrieval_score,
				retrieval_rank: item.retrieval_rank.map(|rank| rank as i32),
				rerank_score: item.rerank_score,
				tie_breaker_score: item.tie_breaker_score,
				final_score: item.final_score,
				boosts: encode_json(&item.boosts, "boosts")?,
				matched_terms: encode_json(&item.matched_terms, "matched_terms")?,
				matched_fields: encode_json(&item.matched_fields, "matched_fields")?,
			});
		}

		let mut builder = sqlx::QueryBuilder::new(
			"INSERT INTO search_trace_items \
             (item_id, trace_id, note_id, rank, retrieval_score, retrieval_rank, rerank_score, \
              tie_breaker_score, final_score, boosts, matched_terms, matched_fields) ",
		);
		builder.push_values(inserts, |mut b, item| {
			b.push_bind(item.item_id)
				.push_bind(trace_id)
				.push_bind(item.note_id)
				.push_bind(item.rank)
				.push_bind(item.retrieval_score)
				.push_bind(item.retrieval_rank)
				.push_bind(item.rerank_score)
				.push_bind(item.tie_breaker_score)
				.push_bind(item.final_score)
				.push_bind(item.boosts)
				.push_bind(item.matched_terms)
				.push_bind(item.matched_fields);
		});
		builder.push(" ON CONFLICT (item_id) DO NOTHING");
		builder.build().execute(&mut *tx).await?;
	}

	tx.commit().await?;
	Ok(())
}

async fn purge_expired_traces(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query("DELETE FROM search_traces WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;
	if result.rows_affected() > 0 {
		info!(count = result.rows_affected(), "Purged expired search traces.");
	}
	Ok(())
}

fn is_not_found_error(err: &qdrant_client::QdrantError) -> bool {
	let message = err.to_string().to_lowercase();
	let point_not_found =
		(message.contains("not found") || message.contains("404")) && message.contains("point");
	let no_point_found = message.contains("no point") && message.contains("found");
	point_not_found || no_point_found
}

async fn fetch_note(db: &Db, note_id: uuid::Uuid) -> Result<Option<MemoryNote>> {
	let note = sqlx::query_as::<_, MemoryNote>(
        "SELECT note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at \
         FROM memory_notes WHERE note_id = $1",
    )
    .bind(note_id)
    .fetch_optional(&db.pool)
    .await?;
	Ok(note)
}

fn note_is_active(note: &MemoryNote, now: OffsetDateTime) -> bool {
	if !note.status.eq_ignore_ascii_case("active") {
		return false;
	}
	if let Some(expires_at) = note.expires_at
		&& expires_at <= now
	{
		return false;
	}
	true
}

async fn ensure_embedding(
	state: &WorkerState,
	note: &MemoryNote,
	embedding_version: &str,
) -> Result<Vec<f32>> {
	let vectors =
		elf_providers::embedding::embed(&state.embedding, std::slice::from_ref(&note.text)).await?;
	let Some(vector) = vectors.into_iter().next() else {
		return Err(eyre!("Embedding provider returned no vectors."));
	};
	validate_vector_dim(&vector, state.qdrant.vector_dim)?;
	insert_embedding(&state.db, note.note_id, embedding_version, vector.len() as i32, &vector)
		.await?;
	Ok(vector)
}

async fn insert_embedding(
	db: &Db,
	note_id: uuid::Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &[f32],
) -> Result<()> {
	let vec_text = format_vector_text(vec);
	sqlx::query(
		"INSERT INTO note_embeddings (note_id, embedding_version, embedding_dim, vec) \
         VALUES ($1, $2, $3, $4::vector) \
         ON CONFLICT (note_id, embedding_version) DO UPDATE \
         SET embedding_dim = EXCLUDED.embedding_dim, vec = EXCLUDED.vec, created_at = now()",
	)
	.bind(note_id)
	.bind(embedding_version)
	.bind(embedding_dim)
	.bind(vec_text)
	.execute(&db.pool)
	.await?;
	Ok(())
}

async fn upsert_qdrant(state: &WorkerState, note: &MemoryNote, vec: &[f32]) -> Result<()> {
	let mut payload_map = HashMap::new();
	payload_map.insert("note_id".to_string(), Value::from(note.note_id.to_string()));
	payload_map.insert("tenant_id".to_string(), Value::from(note.tenant_id.clone()));
	payload_map.insert("project_id".to_string(), Value::from(note.project_id.clone()));
	payload_map.insert("agent_id".to_string(), Value::from(note.agent_id.clone()));
	payload_map.insert("scope".to_string(), Value::from(note.scope.clone()));
	payload_map.insert("status".to_string(), Value::from(note.status.clone()));
	payload_map.insert("type".to_string(), Value::from(note.r#type.clone()));
	payload_map.insert(
		"key".to_string(),
		note.key
			.as_ref()
			.map(|key| Value::from(key.clone()))
			.unwrap_or_else(|| Value::from(JsonValue::Null)),
	);
	payload_map.insert(
		"updated_at".to_string(),
		Value::from(JsonValue::String(format_timestamp(note.updated_at)?)),
	);
	payload_map.insert(
		"expires_at".to_string(),
		Value::from(match note.expires_at {
			Some(ts) => JsonValue::String(format_timestamp(ts)?),
			None => JsonValue::Null,
		}),
	);
	payload_map
		.insert("importance".to_string(), Value::from(JsonValue::from(note.importance as f64)));
	payload_map
		.insert("confidence".to_string(), Value::from(JsonValue::from(note.confidence as f64)));
	payload_map
		.insert("embedding_version".to_string(), Value::from(note.embedding_version.clone()));

	let payload = Payload::from(payload_map);
	let mut vectors = HashMap::new();
	vectors.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec.to_vec()));
	vectors.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(note.text.clone(), BM25_MODEL)),
	);
	let point = PointStruct::new(note.note_id.to_string(), vectors, payload);
	let upsert = UpsertPointsBuilder::new(state.qdrant.collection.clone(), vec![point]).wait(true);
	state.qdrant.client.upsert_points(upsert).await?;
	Ok(())
}

fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	use time::format_description::well_known::Rfc3339;
	ts.format(&Rfc3339).map_err(|_| eyre!("Failed to format timestamp."))
}

fn validate_vector_dim(vec: &[f32], expected_dim: u32) -> Result<()> {
	if vec.len() != expected_dim as usize {
		return Err(eyre!(
			"Embedding dimension {} does not match configured vector_dim {}.",
			vec.len(),
			expected_dim
		));
	}
	Ok(())
}

fn format_vector_text(vec: &[f32]) -> String {
	let mut out = String::from("[");
	for (idx, value) in vec.iter().enumerate() {
		if idx > 0 {
			out.push(',');
		}
		out.push_str(&value.to_string());
	}
	out.push(']');
	out
}

fn encode_json<T: Serialize>(value: &T, label: &str) -> Result<SerdeValue> {
	serde_json::to_value(value).map_err(|err| eyre!("Failed to encode {label}: {err}."))
}

async fn mark_done(db: &Db, outbox_id: uuid::Uuid) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	sqlx::query("UPDATE indexing_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2")
		.bind(now)
		.bind(outbox_id)
		.execute(&db.pool)
		.await?;
	Ok(())
}

async fn mark_trace_done(db: &Db, outbox_id: uuid::Uuid) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	sqlx::query(
		"UPDATE search_trace_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2",
	)
	.bind(now)
	.bind(outbox_id)
	.execute(&db.pool)
	.await?;
	Ok(())
}

async fn mark_failed(
	db: &Db,
	outbox_id: uuid::Uuid,
	attempts: i32,
	err: &color_eyre::Report,
) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	sqlx::query(
		"UPDATE indexing_outbox \
         SET status = 'FAILED', attempts = $1, last_error = $2, available_at = $3, updated_at = $4 \
         WHERE outbox_id = $5",
	)
	.bind(next_attempts)
	.bind(err.to_string())
	.bind(available_at)
	.bind(now)
	.bind(outbox_id)
	.execute(&db.pool)
	.await?;
	Ok(())
}

async fn mark_trace_failed(
	db: &Db,
	outbox_id: uuid::Uuid,
	attempts: i32,
	err: &color_eyre::Report,
) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	sqlx::query(
		"UPDATE search_trace_outbox \
         SET status = 'FAILED', attempts = $1, last_error = $2, available_at = $3, updated_at = $4 \
         WHERE outbox_id = $5",
	)
	.bind(next_attempts)
	.bind(err.to_string())
	.bind(available_at)
	.bind(now)
	.bind(outbox_id)
	.execute(&db.pool)
	.await?;
	Ok(())
}

fn backoff_for_attempt(attempt: i32) -> Duration {
	let attempts = attempt.max(1) as u32;
	let exp = attempts.saturating_sub(1).min(6);
	let base = BASE_BACKOFF_MS.saturating_mul(1 << exp);
	let capped = base.min(MAX_BACKOFF_MS);
	Duration::milliseconds(capped)
}

fn to_std_duration(duration: Duration) -> std::time::Duration {
	let millis = duration.whole_milliseconds();
	if millis <= 0 {
		return std::time::Duration::from_millis(0);
	}
	std::time::Duration::from_millis(millis as u64)
}
