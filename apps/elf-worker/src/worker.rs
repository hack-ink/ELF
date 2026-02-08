use std::{collections::HashMap, time::Duration as StdDuration};

use color_eyre::{Result, eyre};
use qdrant_client::{
	client::Payload,
	qdrant::{
		Condition, DeletePointsBuilder, Document, Filter, PointStruct, UpsertPointsBuilder, Value,
		Vector,
	},
};
use serde::Serialize;
use serde_json::{Value as JsonValue, Value as SerdeValue};
use sqlx::QueryBuilder;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::time as tokio_time;
use uuid::Uuid;

use elf_chunking::{Chunk, ChunkingConfig, Tokenizer};
use elf_providers::embedding;
use elf_storage::{
	db::Db,
	models::{IndexingOutboxEntry, MemoryNote},
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME, QdrantStore},
	queries,
};

const POLL_INTERVAL_MS: i64 = 500;
const CLAIM_LEASE_SECONDS: i64 = 30;
const BASE_BACKOFF_MS: i64 = 500;
const MAX_BACKOFF_MS: i64 = 30_000;
const TRACE_CLEANUP_INTERVAL_SECONDS: i64 = 900;
const TRACE_OUTBOX_LEASE_SECONDS: i64 = 30;
const MAX_OUTBOX_ERROR_CHARS: usize = 1_024;

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
	#[serde(default)]
	chunk_id: Option<uuid::Uuid>,
	rank: u32,
	final_score: f32,
	explain: SerdeValue,
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
	chunk_id: Option<uuid::Uuid>,
	rank: i32,
	final_score: f32,
	explain: SerdeValue,
}

struct ChunkRecord {
	chunk_id: uuid::Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: String,
}

pub struct WorkerState {
	pub db: Db,
	pub qdrant: QdrantStore,
	pub embedding: elf_config::EmbeddingProviderConfig,
	pub chunking: ChunkingConfig,
	pub tokenizer: Tokenizer,
}

pub async fn run_worker(state: WorkerState) -> Result<()> {
	let mut last_trace_cleanup = OffsetDateTime::now_utc();

	loop {
		if let Err(err) = process_indexing_outbox_once(&state).await {
			tracing::error!(error = %err, "Indexing outbox processing failed.");
		}
		if let Err(err) = process_trace_outbox_once(&state).await {
			tracing::error!(error = %err, "Search trace outbox processing failed.");
		}

		let now = OffsetDateTime::now_utc();

		if now - last_trace_cleanup >= Duration::seconds(TRACE_CLEANUP_INTERVAL_SECONDS) {
			if let Err(err) = purge_expired_traces(&state.db, now).await {
				tracing::error!(error = %err, "Search trace cleanup failed.");
			} else {
				last_trace_cleanup = now;
			}
			if let Err(err) = purge_expired_cache(&state.db, now).await {
				tracing::error!(error = %err, "LLM cache cleanup failed.");
			}
			if let Err(err) = purge_expired_search_sessions(&state.db, now).await {
				tracing::error!(error = %err, "Search session cleanup failed.");
			}
		}

		tokio_time::sleep(to_std_duration(Duration::milliseconds(POLL_INTERVAL_MS))).await;
	}
}

fn is_not_found_error(err: &qdrant_client::QdrantError) -> bool {
	let message = err.to_string().to_lowercase();
	let point_not_found =
		(message.contains("not found") || message.contains("404")) && message.contains("point");
	let no_point_found = message.contains("no point") && message.contains("found");
	point_not_found || no_point_found
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

fn build_chunk_records(note_id: uuid::Uuid, chunks: &[Chunk]) -> Result<Vec<ChunkRecord>> {
	let mut records = Vec::with_capacity(chunks.len());

	for chunk in chunks {
		let start_offset = to_i32(chunk.start_offset, "start_offset")?;
		let end_offset = to_i32(chunk.end_offset, "end_offset")?;

		records.push(ChunkRecord {
			chunk_id: chunk_id_for(note_id, chunk.chunk_index),
			chunk_index: chunk.chunk_index,
			start_offset,
			end_offset,
			text: chunk.text.clone(),
		});
	}

	Ok(records)
}

fn chunk_id_for(note_id: uuid::Uuid, chunk_index: i32) -> uuid::Uuid {
	let name = format!("{note_id}:{chunk_index}");

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

fn to_i32(value: usize, label: &str) -> Result<i32> {
	i32::try_from(value)
		.map_err(|_| eyre::eyre!("Chunk {label} offset {value} exceeds supported range."))
}

fn mean_pool(chunks: &[Vec<f32>]) -> Option<Vec<f32>> {
	if chunks.is_empty() {
		return None;
	}

	let dim = chunks[0].len();

	let mut out = vec![0.0_f32; dim];

	for vec in chunks {
		for (idx, value) in vec.iter().enumerate() {
			out[idx] += value;
		}
	}
	for value in &mut out {
		*value /= chunks.len() as f32;
	}

	Some(out)
}

fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	ts.format(&Rfc3339).map_err(|_| eyre::eyre!("Failed to format timestamp."))
}

fn validate_vector_dim(vec: &[f32], expected_dim: u32) -> Result<()> {
	if vec.len() != expected_dim as usize {
		return Err(eyre::eyre!(
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

fn encode_json<T>(value: &T, label: &str) -> Result<SerdeValue>
where
	T: Serialize,
{
	serde_json::to_value(value).map_err(|err| eyre::eyre!("Failed to encode {label}: {err}."))
}

fn sanitize_outbox_error(text: &str) -> String {
	let mut parts = Vec::new();
	let mut redact_next = false;

	for raw in text.split_whitespace() {
		let mut word = raw.to_string();

		if redact_next {
			word = "[REDACTED]".to_string();
			redact_next = false;
		}
		if raw.eq_ignore_ascii_case("bearer") {
			redact_next = true;
		}

		let lowered = raw.to_ascii_lowercase();

		for key in ["api_key", "apikey", "password", "secret", "token"] {
			if lowered.contains(key) && (lowered.contains('=') || lowered.contains(':')) {
				let sep = if raw.contains('=') { '=' } else { ':' };
				let prefix = match raw.split(sep).next() {
					Some(prefix) => prefix,
					None => raw,
				};

				word = format!("{prefix}{sep}[REDACTED]");

				break;
			}
		}

		parts.push(word);
	}

	let mut out = parts.join(" ");

	if out.chars().count() > MAX_OUTBOX_ERROR_CHARS {
		out = out.chars().take(MAX_OUTBOX_ERROR_CHARS).collect();
		out.push_str("...");
	}

	out
}

fn backoff_for_attempt(attempt: i32) -> Duration {
	let attempts = attempt.max(1) as u32;
	let exp = attempts.saturating_sub(1).min(6);
	let base = BASE_BACKOFF_MS.saturating_mul(1 << exp);
	let capped = base.min(MAX_BACKOFF_MS);

	Duration::milliseconds(capped)
}

fn to_std_duration(duration: Duration) -> StdDuration {
	let millis = duration.whole_milliseconds();

	if millis <= 0 {
		return StdDuration::from_millis(0);
	}

	StdDuration::from_millis(millis as u64)
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
		other => Err(eyre::eyre!("Unsupported outbox op: {other}.")),
	};

	match result {
		Ok(()) => {
			mark_done(&state.db, job.outbox_id).await?;
		},
		Err(err) => {
			mark_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
			tracing::error!(error = %err, outbox_id = %job.outbox_id, "Outbox job failed.");
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
			tracing::error!(error = %err, trace_id = %job.trace_id, "Search trace outbox job failed.");
		},
	}

	Ok(())
}

// TODO: Add outbox fetch/update helpers in elf_storage::outbox and use them here.
async fn fetch_next_job(db: &Db, now: OffsetDateTime) -> Result<Option<IndexingOutboxEntry>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query_as!(
		IndexingOutboxEntry,
		"\
SELECT
	outbox_id,
	note_id,
	op,
	embedding_version,
	status,
	attempts,
	last_error,
	available_at,
	created_at,
	updated_at
FROM indexing_outbox
WHERE status IN ('PENDING','FAILED') AND available_at <= $1
ORDER BY available_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED",
		now,
	)
	.fetch_optional(&mut *tx)
	.await?;

	let job = if let Some(mut job) = row {
		let lease_until = now + Duration::seconds(CLAIM_LEASE_SECONDS);
		sqlx::query!(
			"UPDATE indexing_outbox SET available_at = $1, updated_at = $2 WHERE outbox_id = $3",
			lease_until,
			now,
			job.outbox_id,
		)
		.execute(&mut *tx)
		.await?;

		job.available_at = lease_until;
		job.updated_at = now;

		Some(job)
	} else {
		None
	};

	tx.commit().await?;

	Ok(job)
}

async fn fetch_next_trace_job(db: &Db, now: OffsetDateTime) -> Result<Option<TraceOutboxJob>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query_as!(
		TraceOutboxJob,
		"\
SELECT
	outbox_id,
	trace_id,
	payload,
	attempts
FROM search_trace_outbox
WHERE status IN ('PENDING','FAILED') AND available_at <= $1
ORDER BY available_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED",
		now,
	)
	.fetch_optional(&mut *tx)
	.await?;
	let job = if let Some(job) = row {
		let lease_until = now + Duration::seconds(TRACE_OUTBOX_LEASE_SECONDS);
		sqlx::query!(
			"UPDATE search_trace_outbox SET available_at = $1, updated_at = $2 WHERE outbox_id = $3",
			lease_until,
			now,
			job.outbox_id,
		)
		.execute(&mut *tx)
		.await?;

		Some(job)
	} else {
		None
	};

	tx.commit().await?;

	Ok(job)
}

async fn handle_upsert(state: &WorkerState, job: &IndexingOutboxEntry) -> Result<()> {
	let note = fetch_note(&state.db, job.note_id).await?;
	let Some(note) = note else {
		tracing::info!(note_id = %job.note_id, "Note missing for outbox job. Marking done.");

		return Ok(());
	};
	let now = OffsetDateTime::now_utc();

	if !note_is_active(&note, now) {
		tracing::info!(note_id = %job.note_id, "Note inactive or expired. Skipping index.");

		return Ok(());
	}

	let chunks = elf_chunking::split_text(&note.text, &state.chunking, &state.tokenizer);

	if chunks.is_empty() {
		return Err(eyre::eyre!("Chunking produced no chunks."));
	}

	let records = build_chunk_records(note.note_id, &chunks)?;
	let chunk_texts: Vec<String> = records.iter().map(|record| record.text.clone()).collect();
	let chunk_vectors = embedding::embed(&state.embedding, &chunk_texts).await?;

	if chunk_vectors.len() != records.len() {
		return Err(eyre::eyre!(
			"Embedding provider returned {} vectors for {} chunks.",
			chunk_vectors.len(),
			records.len()
		));
	}

	for vector in &chunk_vectors {
		validate_vector_dim(vector, state.qdrant.vector_dim)?;
	}

	{
		let mut tx = state.db.pool.begin().await?;

		queries::delete_note_chunks_tx(&mut tx, note.note_id).await?;

		for record in &records {
			queries::insert_note_chunk_tx(
				&mut tx,
				record.chunk_id,
				note.note_id,
				record.chunk_index,
				record.start_offset,
				record.end_offset,
				record.text.as_str(),
				&job.embedding_version,
			)
			.await?;
		}

		for (record, vector) in records.iter().zip(chunk_vectors.iter()) {
			let vec_text = format_vector_text(vector);

			queries::insert_note_chunk_embedding_tx(
				&mut tx,
				record.chunk_id,
				&job.embedding_version,
				vector.len() as i32,
				vec_text.as_str(),
			)
			.await?;
		}

		let pooled = mean_pool(&chunk_vectors)
			.ok_or_else(|| eyre::eyre!("Cannot pool empty chunk vectors."))?;

		validate_vector_dim(&pooled, state.qdrant.vector_dim)?;
		insert_embedding_tx(
			&mut tx,
			note.note_id,
			&job.embedding_version,
			pooled.len() as i32,
			&pooled,
		)
		.await?;

		tx.commit().await?;
	}
	delete_qdrant_note_points(state, note.note_id).await?;
	upsert_qdrant_chunks(state, &note, &job.embedding_version, &records, &chunk_vectors).await?;

	Ok(())
}

async fn handle_delete(state: &WorkerState, job: &IndexingOutboxEntry) -> Result<()> {
	delete_qdrant_note_points(state, job.note_id).await?;

	Ok(())
}

async fn handle_trace_job(db: &Db, job: &TraceOutboxJob) -> Result<()> {
	let payload: TracePayload = serde_json::from_value(job.payload.clone())?;
	let trace = payload.trace;
	let trace_id = trace.trace_id;
	let expanded_queries_json = encode_json(&trace.expanded_queries, "expanded_queries")?;
	let allowed_scopes_json = encode_json(&trace.allowed_scopes, "allowed_scopes")?;

	let mut tx = db.pool.begin().await?;

	sqlx::query!(
		"\
INSERT INTO search_traces (
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	expansion_mode,
	expanded_queries,
	allowed_scopes,
	candidate_count,
	top_k,
	config_snapshot,
	trace_version,
	created_at,
	expires_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15
	)
	ON CONFLICT (trace_id) DO NOTHING",
		trace_id,
		trace.tenant_id.as_str(),
		trace.project_id.as_str(),
		trace.agent_id.as_str(),
		trace.read_profile.as_str(),
		trace.query.as_str(),
		trace.expansion_mode.as_str(),
		expanded_queries_json,
		allowed_scopes_json,
		trace.candidate_count as i32,
		trace.top_k as i32,
		trace.config_snapshot,
		trace.trace_version,
		trace.created_at,
		trace.expires_at,
	)
	.execute(&mut *tx)
	.await?;

	if !payload.items.is_empty() {
		let mut inserts = Vec::with_capacity(payload.items.len());

		for item in payload.items {
			inserts.push(TraceItemInsert {
				item_id: item.item_id,
				note_id: item.note_id,
				chunk_id: item.chunk_id,
				rank: item.rank as i32,
				final_score: item.final_score,
				explain: item.explain,
			});
		}

		let mut builder = QueryBuilder::new(
			"\
INSERT INTO search_trace_items (
	item_id,
	trace_id,
	note_id,
	chunk_id,
	rank,
	final_score,
	explain
) ",
		);
		builder.push_values(inserts, |mut b, item| {
			b.push_bind(item.item_id)
				.push_bind(trace_id)
				.push_bind(item.note_id)
				.push_bind(item.chunk_id)
				.push_bind(item.rank)
				.push_bind(item.final_score)
				.push_bind(item.explain);
		});
		builder.push(" ON CONFLICT (item_id) DO NOTHING");
		builder.build().execute(&mut *tx).await?;
	}

	tx.commit().await?;

	Ok(())
}

async fn purge_expired_traces(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query!("DELETE FROM search_traces WHERE expires_at <= $1", now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search traces.");
	}

	Ok(())
}

async fn purge_expired_cache(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result =
		sqlx::query!("DELETE FROM llm_cache WHERE expires_at <= $1", now).execute(&db.pool).await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired LLM cache entries.");
	}

	Ok(())
}

async fn purge_expired_search_sessions(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query!("DELETE FROM search_sessions WHERE expires_at <= $1", now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search sessions.");
	}

	Ok(())
}

async fn fetch_note(db: &Db, note_id: uuid::Uuid) -> Result<Option<MemoryNote>> {
	let note =
		sqlx::query_as!(MemoryNote, "SELECT * FROM memory_notes WHERE note_id = $1", note_id,)
			.fetch_optional(&db.pool)
			.await?;

	Ok(note)
}

async fn insert_embedding_tx(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	note_id: uuid::Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &[f32],
) -> Result<()> {
	let vec_text = format_vector_text(vec);

	sqlx::query!(
		"\
	INSERT INTO note_embeddings (
		note_id,
	embedding_version,
		embedding_dim,
		vec
	)
	VALUES ($1, $2, $3, $4::text::vector)
	ON CONFLICT (note_id, embedding_version) DO UPDATE
	SET
			embedding_dim = EXCLUDED.embedding_dim,
			vec = EXCLUDED.vec,
		created_at = now()",
		note_id,
		embedding_version,
		embedding_dim,
		vec_text.as_str(),
	)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

async fn delete_qdrant_note_points(state: &WorkerState, note_id: uuid::Uuid) -> Result<()> {
	let filter = Filter::must([Condition::matches("note_id", note_id.to_string())]);
	let delete =
		DeletePointsBuilder::new(state.qdrant.collection.clone()).points(filter).wait(true);
	match state.qdrant.client.delete_points(delete).await {
		Ok(_) => {},
		Err(err) =>
			if is_not_found_error(&err) {
				tracing::info!(note_id = %note_id, "Qdrant points missing during delete.");
			} else {
				return Err(eyre::eyre!(err.to_string()));
			},
	}

	Ok(())
}

async fn upsert_qdrant_chunks(
	state: &WorkerState,
	note: &MemoryNote,
	embedding_version: &str,
	records: &[ChunkRecord],
	vectors: &[Vec<f32>],
) -> Result<()> {
	let mut points = Vec::with_capacity(records.len());

	for (record, vec) in records.iter().zip(vectors.iter()) {
		let mut payload_map = HashMap::new();

		payload_map.insert("note_id".to_string(), Value::from(note.note_id.to_string()));
		payload_map.insert("chunk_id".to_string(), Value::from(record.chunk_id.to_string()));
		payload_map.insert("chunk_index".to_string(), Value::from(record.chunk_index as i64));
		payload_map.insert("start_offset".to_string(), Value::from(record.start_offset as i64));
		payload_map.insert("end_offset".to_string(), Value::from(record.end_offset as i64));
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
			.insert("embedding_version".to_string(), Value::from(embedding_version.to_string()));

		let payload = Payload::from(payload_map);
		let mut vector_map = HashMap::new();

		vector_map.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec.to_vec()));
		vector_map.insert(
			BM25_VECTOR_NAME.to_string(),
			Vector::from(Document::new(record.text.clone(), BM25_MODEL)),
		);
		let point = PointStruct::new(record.chunk_id.to_string(), vector_map, payload);

		points.push(point);
	}

	let upsert = UpsertPointsBuilder::new(state.qdrant.collection.clone(), points).wait(true);
	state.qdrant.client.upsert_points(upsert).await?;

	Ok(())
}

async fn mark_done(db: &Db, outbox_id: uuid::Uuid) -> Result<()> {
	let now = OffsetDateTime::now_utc();

	sqlx::query!(
		"UPDATE indexing_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2",
		now,
		outbox_id,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}

async fn mark_trace_done(db: &Db, outbox_id: uuid::Uuid) -> Result<()> {
	let now = OffsetDateTime::now_utc();

	sqlx::query!(
		"UPDATE search_trace_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2",
		now,
		outbox_id,
	)
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
	let error_text = sanitize_outbox_error(&err.to_string());

	sqlx::query!(
		"\
UPDATE indexing_outbox
SET status = 'FAILED',
	attempts = $1,
	last_error = $2,
	available_at = $3,
	updated_at = $4
WHERE outbox_id = $5",
		next_attempts,
		error_text,
		available_at,
		now,
		outbox_id,
	)
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
	let error_text = sanitize_outbox_error(&err.to_string());

	sqlx::query!(
		"\
UPDATE search_trace_outbox
SET status = 'FAILED',
	attempts = $1,
	last_error = $2,
	available_at = $3,
	updated_at = $4
WHERE outbox_id = $5",
		next_attempts,
		error_text,
		available_at,
		now,
		outbox_id,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pooled_vector_is_mean_of_chunks() {
		let chunks = vec![vec![1.0_f32, 3.0_f32], vec![3.0_f32, 5.0_f32]];
		let pooled = mean_pool(&chunks).expect("Expected pooled vector.");
		assert_eq!(pooled, vec![2.0_f32, 4.0_f32]);
	}
}
