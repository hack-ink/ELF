use std::collections::HashMap;

use qdrant_client::{
	QdrantError,
	client::Payload,
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
use elf_providers::embedding;
use elf_storage::{
	db::Db,
	doc_outbox, docs,
	models::{DocIndexingOutboxEntry, IndexingOutboxEntry, MemoryNote, TraceOutboxJob},
	outbox,
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

pub struct WorkerState {
	pub db: Db,
	pub qdrant: QdrantStore,
	pub docs_qdrant: QdrantStore,
	pub embedding: EmbeddingProviderConfig,
	pub chunking: ChunkingConfig,
	pub tokenizer: Tokenizer,
}

#[derive(Debug, Deserialize)]
struct TracePayload {
	trace: TraceRecord,
	items: Vec<TraceItemRecord>,
	#[serde(default)]
	candidates: Vec<TraceCandidateRecord>,
	#[serde(default)]
	stages: Vec<TraceTrajectoryStageRecord>,
}

#[derive(Debug, Deserialize)]
struct TraceRecord {
	trace_id: Uuid,
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
	config_snapshot: Value,
	trace_version: i32,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
struct TraceItemRecord {
	item_id: Uuid,
	note_id: Uuid,
	chunk_id: Option<Uuid>,
	rank: u32,
	final_score: f32,
	explain: Value,
}

#[derive(Debug, Deserialize)]
struct TraceCandidateRecord {
	candidate_id: Uuid,
	note_id: Uuid,
	chunk_id: Uuid,
	#[serde(default)]
	chunk_index: i32,
	#[serde(default)]
	snippet: String,
	#[serde(default)]
	candidate_snapshot: Value,
	retrieval_rank: u32,
	rerank_score: f32,
	note_scope: String,
	note_importance: f32,
	note_updated_at: OffsetDateTime,
	#[serde(default)]
	note_hit_count: i64,
	note_last_hit_at: Option<OffsetDateTime>,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
struct TraceTrajectoryStageRecord {
	stage_id: Uuid,
	stage_order: u32,
	stage_name: String,
	stage_payload: Value,
	created_at: OffsetDateTime,
	#[serde(default)]
	items: Vec<TraceTrajectoryStageItemRecord>,
}

#[derive(Debug, Deserialize)]
struct TraceTrajectoryStageItemRecord {
	id: Uuid,
	item_id: Option<Uuid>,
	note_id: Option<Uuid>,
	chunk_id: Option<Uuid>,
	metrics: Value,
}

struct TraceItemInsert {
	item_id: Uuid,
	note_id: Uuid,
	chunk_id: Option<Uuid>,
	rank: i32,
	final_score: f32,
	explain: Value,
}

struct TraceCandidateInsert {
	candidate_id: Uuid,
	note_id: Uuid,
	chunk_id: Uuid,
	chunk_index: i32,
	snippet: String,
	candidate_snapshot: Value,
	retrieval_rank: i32,
	rerank_score: f32,
	note_scope: String,
	note_importance: f32,
	note_updated_at: OffsetDateTime,
	note_hit_count: i64,
	note_last_hit_at: Option<OffsetDateTime>,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

struct TraceStageInsert {
	stage_id: Uuid,
	stage_order: i32,
	stage_name: String,
	stage_payload: Value,
	created_at: OffsetDateTime,
}

struct TraceStageItemInsert {
	id: Uuid,
	stage_id: Uuid,
	item_id: Option<Uuid>,
	note_id: Option<Uuid>,
	chunk_id: Option<Uuid>,
	metrics: Value,
}

struct ChunkRecord {
	chunk_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: String,
}

#[derive(Debug, FromRow)]
struct NoteFieldRow {
	field_id: Uuid,
	text: String,
}

#[derive(Debug, FromRow)]
struct DocChunkIndexRow {
	doc_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	scope: String,
	doc_type: String,
	status: String,
	updated_at: OffsetDateTime,
	content_hash: String,
	chunk_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	chunk_text: String,
	chunk_hash: String,
}

pub async fn run_worker(state: WorkerState) -> Result<()> {
	let mut last_trace_cleanup = OffsetDateTime::now_utc();

	loop {
		if let Err(err) = process_indexing_outbox_once(&state).await {
			tracing::error!(error = %err, "Indexing outbox processing failed.");
		}
		if let Err(err) = process_doc_indexing_outbox_once(&state).await {
			tracing::error!(error = %err, "Doc indexing outbox processing failed.");
		}
		if let Err(err) = process_trace_outbox_once(&state).await {
			tracing::error!(error = %err, "Search trace outbox processing failed.");
		}

		let now = OffsetDateTime::now_utc();

		if now - last_trace_cleanup >= time::Duration::seconds(TRACE_CLEANUP_INTERVAL_SECONDS) {
			if let Err(err) = purge_expired_trace_candidates(&state.db, now).await {
				tracing::error!(error = %err, "Search trace candidate cleanup failed.");
			}
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

		tokio::time::sleep(to_std_duration(time::Duration::milliseconds(POLL_INTERVAL_MS))).await;
	}
}

fn is_not_found_error(err: &QdrantError) -> bool {
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

fn build_chunk_records(note_id: Uuid, chunks: &[Chunk]) -> Result<Vec<ChunkRecord>> {
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

fn chunk_id_for(note_id: Uuid, chunk_index: i32) -> Uuid {
	let name = format!("{note_id}:{chunk_index}");

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

fn to_i32(value: usize, label: &str) -> Result<i32> {
	i32::try_from(value).map_err(|_| {
		Error::Validation(format!("Chunk {label} offset {value} exceeds supported range."))
	})
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
	ts.format(&Rfc3339).map_err(|_| Error::Message("Failed to format timestamp.".to_string()))
}

fn validate_vector_dim(vec: &[f32], expected_dim: u32) -> Result<()> {
	if vec.len() != expected_dim as usize {
		return Err(Error::Validation(format!(
			"Embedding dimension {} does not match configured vector_dim {}.",
			vec.len(),
			expected_dim
		)));
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

fn encode_json<T>(value: &T, label: &str) -> Result<Value>
where
	T: Serialize,
{
	serde_json::to_value(value)
		.map_err(|err| Error::Message(format!("Failed to encode {label}: {err}.")))
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

fn backoff_for_attempt(attempt: i32) -> time::Duration {
	let attempts = attempt.max(1) as u32;
	let exp = attempts.saturating_sub(1).min(6);
	let base = BASE_BACKOFF_MS.saturating_mul(1 << exp);
	let capped = base.min(MAX_BACKOFF_MS);

	time::Duration::milliseconds(capped)
}

fn to_std_duration(duration: time::Duration) -> std::time::Duration {
	let millis = duration.whole_milliseconds();

	if millis <= 0 {
		return std::time::Duration::from_millis(0);
	}

	std::time::Duration::from_millis(millis as u64)
}

async fn process_indexing_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job = outbox::claim_next_indexing_outbox_job(&state.db, now, CLAIM_LEASE_SECONDS).await?;
	let Some(job) = job else { return Ok(()) };
	let result = match job.op.as_str() {
		"UPSERT" => handle_upsert(state, &job).await,
		"DELETE" => handle_delete(state, &job).await,
		other => Err(Error::Validation(format!("Unsupported outbox op: {other}."))),
	};

	match result {
		Ok(()) => {
			outbox::mark_indexing_outbox_done(&state.db, job.outbox_id, OffsetDateTime::now_utc())
				.await?;
		},
		Err(err) => {
			tracing::error!(error = %err, outbox_id = %job.outbox_id, "Outbox job failed.");

			mark_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
		},
	}

	Ok(())
}

async fn process_doc_indexing_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job =
		doc_outbox::claim_next_doc_indexing_outbox_job(&state.db, now, CLAIM_LEASE_SECONDS).await?;
	let Some(job) = job else { return Ok(()) };
	let result = match job.op.as_str() {
		"UPSERT" => handle_doc_upsert(state, &job).await,
		"DELETE" => handle_doc_delete(state, &job).await,
		other => Err(Error::Validation(format!("Unsupported doc outbox op: {other}."))),
	};

	match result {
		Ok(()) => {
			doc_outbox::mark_doc_indexing_outbox_done(
				&state.db,
				job.outbox_id,
				OffsetDateTime::now_utc(),
			)
			.await?;
		},
		Err(err) => {
			tracing::error!(error = %err, outbox_id = %job.outbox_id, "Doc outbox job failed.");

			mark_doc_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
		},
	}

	Ok(())
}

async fn process_trace_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job =
		outbox::claim_next_trace_outbox_job(&state.db, now, TRACE_OUTBOX_LEASE_SECONDS).await?;
	let Some(job) = job else { return Ok(()) };
	let result = handle_trace_job(&state.db, &job).await;

	match result {
		Ok(()) => {
			outbox::mark_trace_outbox_done(&state.db, job.outbox_id, OffsetDateTime::now_utc())
				.await?;
		},
		Err(err) => {
			tracing::error!(error = %err, trace_id = %job.trace_id, "Search trace outbox job failed.");

			mark_trace_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
		},
	}

	Ok(())
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

	let fields = fetch_note_fields(&state.db, note.note_id).await?;
	let chunks = elf_chunking::split_text(&note.text, &state.chunking, &state.tokenizer);

	if chunks.is_empty() {
		return Err(Error::Validation("Chunking produced no chunks.".to_string()));
	}

	let records = build_chunk_records(note.note_id, &chunks)?;
	let chunk_texts: Vec<String> = records.iter().map(|record| record.text.clone()).collect();
	let field_texts: Vec<String> = fields.iter().map(|field| field.text.clone()).collect();
	let mut embed_inputs = Vec::with_capacity(chunk_texts.len() + field_texts.len());

	embed_inputs.extend(chunk_texts);
	embed_inputs.extend(field_texts);

	let vectors = embedding::embed(&state.embedding, &embed_inputs)
		.await
		.map_err(|err| Error::Message(err.to_string()))?;

	if vectors.len() != records.len() + fields.len() {
		return Err(Error::Validation(format!(
			"Embedding provider returned {} vectors for {} items.",
			vectors.len(),
			records.len() + fields.len()
		)));
	}

	let (chunk_vectors, field_vectors) = vectors.split_at(records.len());

	for vector in chunk_vectors.iter().chain(field_vectors.iter()) {
		validate_vector_dim(vector, state.qdrant.vector_dim)?;
	}

	{
		let mut tx = state.db.pool.begin().await?;

		queries::delete_note_chunks(&mut *tx, note.note_id).await?;

		for record in &records {
			queries::insert_note_chunk(
				&mut *tx,
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

			queries::insert_note_chunk_embedding(
				&mut *tx,
				record.chunk_id,
				&job.embedding_version,
				vector.len() as i32,
				vec_text.as_str(),
			)
			.await?;
		}

		let pooled = mean_pool(chunk_vectors)
			.ok_or_else(|| Error::Message("Cannot pool empty chunk vectors.".to_string()))?;

		validate_vector_dim(&pooled, state.qdrant.vector_dim)?;
		insert_embedding_tx(
			&mut *tx,
			note.note_id,
			&job.embedding_version,
			pooled.len() as i32,
			&pooled,
		)
		.await?;

		for (field, vector) in fields.iter().zip(field_vectors.iter()) {
			insert_note_field_embedding_tx(
				&mut *tx,
				field.field_id,
				&job.embedding_version,
				vector.len() as i32,
				vector,
			)
			.await?;
		}

		tx.commit().await?;
	}

	delete_qdrant_note_points(state, note.note_id).await?;
	upsert_qdrant_chunks(state, &note, &job.embedding_version, &records, chunk_vectors).await?;

	Ok(())
}

async fn handle_delete(state: &WorkerState, job: &IndexingOutboxEntry) -> Result<()> {
	delete_qdrant_note_points(state, job.note_id).await?;

	Ok(())
}

async fn fetch_doc_chunk_index_row(db: &Db, chunk_id: Uuid) -> Result<Option<DocChunkIndexRow>> {
	let row = sqlx::query_as::<_, DocChunkIndexRow>(
		"\
SELECT
\td.doc_id,
\td.tenant_id,
\td.project_id,
\td.agent_id,
\td.scope,
\td.doc_type,
\td.status,
\td.updated_at,
\td.content_hash,
\tc.chunk_id,
\tc.chunk_index,
\tc.start_offset,
\tc.end_offset,
\tc.chunk_text,
\tc.chunk_hash
FROM doc_chunks c
JOIN doc_documents d ON d.doc_id = c.doc_id
WHERE c.chunk_id = $1
LIMIT 1",
	)
	.bind(chunk_id)
	.fetch_optional(&db.pool)
	.await?;

	Ok(row)
}

async fn handle_doc_upsert(state: &WorkerState, job: &DocIndexingOutboxEntry) -> Result<()> {
	let row = fetch_doc_chunk_index_row(&state.db, job.chunk_id).await?;
	let Some(row) = row else {
		tracing::info!(chunk_id = %job.chunk_id, "Doc chunk missing for outbox job. Marking done.");

		return Ok(());
	};

	if !row.status.eq_ignore_ascii_case("active") {
		tracing::info!(doc_id = %row.doc_id, chunk_id = %row.chunk_id, "Doc inactive. Skipping index.");

		return Ok(());
	}

	let vectors = embedding::embed(&state.embedding, std::slice::from_ref(&row.chunk_text))
		.await
		.map_err(|err| Error::Message(err.to_string()))?;
	let vector = vectors
		.first()
		.ok_or_else(|| Error::Validation("Embedding provider returned no vectors.".to_string()))?;

	validate_vector_dim(vector, state.docs_qdrant.vector_dim)?;

	{
		let vec_text = format_vector_text(vector);
		let mut tx = state.db.pool.begin().await?;

		docs::insert_doc_chunk_embedding(
			&mut *tx,
			row.chunk_id,
			&job.embedding_version,
			vector.len() as i32,
			vec_text.as_str(),
		)
		.await?;

		tx.commit().await?;
	}

	upsert_qdrant_doc_chunk(state, &row, &job.embedding_version, vector).await?;

	Ok(())
}

async fn handle_doc_delete(state: &WorkerState, job: &DocIndexingOutboxEntry) -> Result<()> {
	let filter = Filter::must([Condition::matches("chunk_id", job.chunk_id.to_string())]);
	let delete =
		DeletePointsBuilder::new(state.docs_qdrant.collection.clone()).points(filter).wait(true);

	state.docs_qdrant.client.delete_points(delete).await?;

	Ok(())
}

async fn upsert_qdrant_doc_chunk(
	state: &WorkerState,
	row: &DocChunkIndexRow,
	embedding_version: &str,
	vec: &[f32],
) -> Result<()> {
	let mut payload = Payload::new();

	payload.insert("doc_id", row.doc_id.to_string());
	payload.insert("chunk_id", row.chunk_id.to_string());
	payload.insert("chunk_index", row.chunk_index as i64);
	payload.insert("start_offset", row.start_offset as i64);
	payload.insert("end_offset", row.end_offset as i64);
	payload.insert("tenant_id", row.tenant_id.clone());
	payload.insert("project_id", row.project_id.clone());
	payload.insert("agent_id", row.agent_id.clone());
	payload.insert("scope", row.scope.clone());
	payload.insert("doc_type", row.doc_type.clone());
	payload.insert("status", row.status.clone());
	payload.insert("updated_at", Value::String(format_timestamp(row.updated_at)?));
	payload.insert("embedding_version", embedding_version.to_string());
	payload.insert("content_hash", row.content_hash.clone());
	payload.insert("chunk_hash", row.chunk_hash.clone());

	let mut vector_map = HashMap::new();

	vector_map.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec.to_vec()));
	vector_map.insert(
		BM25_VECTOR_NAME.to_string(),
		Vector::from(Document::new(row.chunk_text.clone(), BM25_MODEL)),
	);

	let point = PointStruct::new(row.chunk_id.to_string(), vector_map, payload);
	let upsert =
		UpsertPointsBuilder::new(state.docs_qdrant.collection.clone(), vec![point]).wait(true);

	state.docs_qdrant.client.upsert_points(upsert).await?;

	Ok(())
}

async fn handle_trace_job(db: &Db, job: &TraceOutboxJob) -> Result<()> {
	let payload: TracePayload = serde_json::from_value(job.payload.clone())?;
	let TracePayload { trace, items, candidates, stages } = payload;
	let trace_id = trace.trace_id;
	let expanded_queries_json = encode_json(&trace.expanded_queries, "expanded_queries")?;
	let allowed_scopes_json = encode_json(&trace.allowed_scopes, "allowed_scopes")?;
	let mut tx = db.pool.begin().await?;

	insert_trace_tx(&mut *tx, trace_id, &trace, expanded_queries_json, allowed_scopes_json).await?;
	insert_trace_items_tx(&mut *tx, trace_id, items).await?;
	insert_trace_stages_tx(&mut tx, trace_id, stages).await?;
	insert_trace_candidates_tx(&mut *tx, trace_id, candidates).await?;

	tx.commit().await?;

	Ok(())
}

async fn insert_trace_stages_tx(
	executor: &mut PgConnection,
	trace_id: Uuid,
	stages: Vec<TraceTrajectoryStageRecord>,
) -> Result<()> {
	if stages.is_empty() {
		return Ok(());
	}

	let mut stage_inserts = Vec::with_capacity(stages.len());
	let mut item_inserts = Vec::new();

	for stage in stages {
		stage_inserts.push(TraceStageInsert {
			stage_id: stage.stage_id,
			stage_order: stage.stage_order as i32,
			stage_name: stage.stage_name,
			stage_payload: stage.stage_payload,
			created_at: stage.created_at,
		});

		for item in stage.items {
			item_inserts.push(TraceStageItemInsert {
				id: item.id,
				stage_id: stage.stage_id,
				item_id: item.item_id,
				note_id: item.note_id,
				chunk_id: item.chunk_id,
				metrics: item.metrics,
			});
		}
	}

	let mut stage_builder = QueryBuilder::new(
		"\
	INSERT INTO search_trace_stages (
		stage_id,
		trace_id,
		stage_order,
		stage_name,
		stage_payload,
		created_at
	) ",
	);

	stage_builder.push_values(stage_inserts, |mut b, stage| {
		b.push_bind(stage.stage_id)
			.push_bind(trace_id)
			.push_bind(stage.stage_order)
			.push_bind(stage.stage_name)
			.push_bind(stage.stage_payload)
			.push_bind(stage.created_at);
	});
	stage_builder.push(" ON CONFLICT (stage_id) DO NOTHING");
	stage_builder.build().execute(&mut *executor).await?;

	if item_inserts.is_empty() {
		return Ok(());
	}

	let mut item_builder = QueryBuilder::new(
		"\
	INSERT INTO search_trace_stage_items (
		id,
		stage_id,
		item_id,
		note_id,
		chunk_id,
		metrics
	) ",
	);

	item_builder.push_values(item_inserts, |mut b, item| {
		b.push_bind(item.id)
			.push_bind(item.stage_id)
			.push_bind(item.item_id)
			.push_bind(item.note_id)
			.push_bind(item.chunk_id)
			.push_bind(item.metrics);
	});
	item_builder.push(" ON CONFLICT (id) DO NOTHING");
	item_builder.build().execute(executor).await?;

	Ok(())
}

async fn insert_trace_tx<'e, E>(
	executor: E,
	trace_id: Uuid,
	trace: &TraceRecord,
	expanded_queries_json: Value,
	allowed_scopes_json: Value,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"INSERT INTO search_traces (
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
	)
	.bind(trace_id)
	.bind(trace.tenant_id.as_str())
	.bind(trace.project_id.as_str())
	.bind(trace.agent_id.as_str())
	.bind(trace.read_profile.as_str())
	.bind(trace.query.as_str())
	.bind(trace.expansion_mode.as_str())
	.bind(expanded_queries_json)
	.bind(allowed_scopes_json)
	.bind(trace.candidate_count as i32)
	.bind(trace.top_k as i32)
	.bind(trace.config_snapshot.clone())
	.bind(trace.trace_version)
	.bind(trace.created_at)
	.bind(trace.expires_at)
	.execute(executor)
	.await?;

	Ok(())
}

async fn insert_trace_items_tx<'e, E>(
	executor: E,
	trace_id: Uuid,
	items: Vec<TraceItemRecord>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if items.is_empty() {
		return Ok(());
	}

	let mut inserts = Vec::with_capacity(items.len());

	for item in items {
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
	builder.build().execute(executor).await?;

	Ok(())
}

async fn insert_trace_candidates_tx<'e, E>(
	executor: E,
	trace_id: Uuid,
	candidates: Vec<TraceCandidateRecord>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if candidates.is_empty() {
		return Ok(());
	}

	let mut inserts = Vec::with_capacity(candidates.len());

	for candidate in candidates {
		inserts.push(TraceCandidateInsert {
			candidate_id: candidate.candidate_id,
			note_id: candidate.note_id,
			chunk_id: candidate.chunk_id,
			chunk_index: candidate.chunk_index,
			snippet: candidate.snippet,
			candidate_snapshot: candidate.candidate_snapshot,
			retrieval_rank: candidate.retrieval_rank as i32,
			rerank_score: candidate.rerank_score,
			note_scope: candidate.note_scope,
			note_importance: candidate.note_importance,
			note_updated_at: candidate.note_updated_at,
			note_hit_count: candidate.note_hit_count,
			note_last_hit_at: candidate.note_last_hit_at,
			created_at: candidate.created_at,
			expires_at: candidate.expires_at,
		});
	}

	let mut builder = QueryBuilder::new(
		"\
INSERT INTO search_trace_candidates (
	candidate_id,
	trace_id,
	note_id,
	chunk_id,
	chunk_index,
	snippet,
	candidate_snapshot,
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at,
	note_hit_count,
	note_last_hit_at,
	created_at,
	expires_at
) ",
	);

	builder.push_values(inserts, |mut b, candidate| {
		b.push_bind(candidate.candidate_id)
			.push_bind(trace_id)
			.push_bind(candidate.note_id)
			.push_bind(candidate.chunk_id)
			.push_bind(candidate.chunk_index)
			.push_bind(candidate.snippet)
			.push_bind(candidate.candidate_snapshot)
			.push_bind(candidate.retrieval_rank)
			.push_bind(candidate.rerank_score)
			.push_bind(candidate.note_scope)
			.push_bind(candidate.note_importance)
			.push_bind(candidate.note_updated_at)
			.push_bind(candidate.note_hit_count)
			.push_bind(candidate.note_last_hit_at)
			.push_bind(candidate.created_at)
			.push_bind(candidate.expires_at);
	});
	builder.push(" ON CONFLICT (candidate_id) DO NOTHING");
	builder.build().execute(executor).await?;

	Ok(())
}

async fn purge_expired_trace_candidates(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query("DELETE FROM search_trace_candidates WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search trace candidates.");
	}

	Ok(())
}

async fn purge_expired_traces(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query("DELETE FROM search_traces WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search traces.");
	}

	Ok(())
}

async fn purge_expired_cache(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query("DELETE FROM llm_cache WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired LLM cache entries.");
	}

	Ok(())
}

async fn purge_expired_search_sessions(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query("DELETE FROM search_sessions WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search sessions.");
	}

	Ok(())
}

async fn fetch_note(db: &Db, note_id: Uuid) -> Result<Option<MemoryNote>> {
	let note = sqlx::query_as::<_, MemoryNote>("SELECT * FROM memory_notes WHERE note_id = $1")
		.bind(note_id)
		.fetch_optional(&db.pool)
		.await?;

	Ok(note)
}

async fn fetch_note_fields(db: &Db, note_id: Uuid) -> Result<Vec<NoteFieldRow>> {
	let rows = sqlx::query_as::<_, NoteFieldRow>(
		"\
SELECT field_id, text
FROM memory_note_fields
WHERE note_id = $1
ORDER BY field_kind ASC, item_index ASC",
	)
	.bind(note_id)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

async fn insert_embedding_tx<'e, E>(
	executor: E,
	note_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &[f32],
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let vec_text = format_vector_text(vec);

	sqlx::query(
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
	)
	.bind(note_id)
	.bind(embedding_version)
	.bind(embedding_dim)
	.bind(vec_text.as_str())
	.execute(executor)
	.await?;

	Ok(())
}

async fn insert_note_field_embedding_tx<'e, E>(
	executor: E,
	field_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &[f32],
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let vec_text = format_vector_text(vec);

	sqlx::query(
		"\
INSERT INTO note_field_embeddings (
	field_id,
	embedding_version,
	embedding_dim,
	vec
)
VALUES ($1, $2, $3, $4::text::vector)
ON CONFLICT (field_id, embedding_version) DO UPDATE
SET
	embedding_dim = EXCLUDED.embedding_dim,
	vec = EXCLUDED.vec,
	created_at = now()",
	)
	.bind(field_id)
	.bind(embedding_version)
	.bind(embedding_dim)
	.bind(vec_text.as_str())
	.execute(executor)
	.await?;

	Ok(())
}

async fn delete_qdrant_note_points(state: &WorkerState, note_id: Uuid) -> Result<()> {
	let filter = Filter::must([Condition::matches("note_id", note_id.to_string())]);
	let delete =
		DeletePointsBuilder::new(state.qdrant.collection.clone()).points(filter).wait(true);

	match state.qdrant.client.delete_points(delete).await {
		Ok(_) => {},
		Err(err) =>
			if is_not_found_error(&err) {
				tracing::info!(note_id = %note_id, "Qdrant points missing during delete.");
			} else {
				return Err(err.into());
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
		let mut payload = Payload::new();

		payload.insert("note_id", note.note_id.to_string());
		payload.insert("chunk_id", record.chunk_id.to_string());
		payload.insert("chunk_index", record.chunk_index as i64);
		payload.insert("start_offset", record.start_offset as i64);
		payload.insert("end_offset", record.end_offset as i64);
		payload.insert("tenant_id", note.tenant_id.clone());
		payload.insert("project_id", note.project_id.clone());
		payload.insert("agent_id", note.agent_id.clone());
		payload.insert("scope", note.scope.clone());
		payload.insert("status", note.status.clone());
		payload.insert("type", note.r#type.clone());

		match note.key.as_ref() {
			Some(key) => payload.insert("key", key.clone()),
			None => payload.insert("key", Value::Null),
		}

		payload.insert("updated_at", Value::String(format_timestamp(note.updated_at)?));
		payload.insert(
			"expires_at",
			match note.expires_at {
				Some(ts) => Value::String(format_timestamp(ts)?),
				None => Value::Null,
			},
		);
		payload.insert("importance", Value::from(note.importance as f64));
		payload.insert("confidence", Value::from(note.confidence as f64));
		payload.insert("embedding_version", embedding_version.to_string());

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

async fn mark_failed(db: &Db, outbox_id: Uuid, attempts: i32, err: &Error) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	let error_text = sanitize_outbox_error(&err.to_string());

	outbox::mark_indexing_outbox_failed(
		db,
		outbox_id,
		next_attempts,
		error_text.as_str(),
		available_at,
		now,
	)
	.await?;

	Ok(())
}

async fn mark_doc_failed(db: &Db, outbox_id: Uuid, attempts: i32, err: &Error) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	let error_text = sanitize_outbox_error(&err.to_string());

	doc_outbox::mark_doc_indexing_outbox_failed(
		db,
		outbox_id,
		next_attempts,
		error_text.as_str(),
		available_at,
		now,
	)
	.await?;

	Ok(())
}

async fn mark_trace_failed(db: &Db, outbox_id: Uuid, attempts: i32, err: &Error) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	let error_text = sanitize_outbox_error(&err.to_string());

	outbox::mark_trace_outbox_failed(
		db,
		outbox_id,
		next_attempts,
		error_text.as_str(),
		available_at,
		now,
	)
	.await?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::worker::mean_pool;

	#[test]
	fn pooled_vector_is_mean_of_chunks() {
		let chunks = vec![vec![1.0_f32, 3.0_f32], vec![3.0_f32, 5.0_f32]];
		let pooled = mean_pool(&chunks).expect("Expected pooled vector.");

		assert_eq!(pooled, vec![2.0_f32, 4.0_f32]);
	}
}
