use std::collections::HashMap;

use color_eyre::{eyre::eyre, Result};
use elf_storage::db::Db;
use elf_storage::models::{IndexingOutboxEntry, MemoryNote};
use elf_storage::qdrant::QdrantStore;
use qdrant_client::client::Payload;
use qdrant_client::qdrant::{DeletePointsBuilder, PointStruct, UpsertPointsBuilder, Value};
use serde_json::Value as JsonValue;
use sqlx::Row;
use time::{Duration, OffsetDateTime};
use tracing::{error, info};

const POLL_INTERVAL_MS: i64 = 500;
const CLAIM_LEASE_SECONDS: i64 = 30;
const BASE_BACKOFF_MS: i64 = 500;
const MAX_BACKOFF_MS: i64 = 30_000;

pub struct WorkerState {
    pub db: Db,
    pub qdrant: QdrantStore,
    pub embedding: elf_config::ProviderConfig,
}

// TODO: Add integration tests that exercise the worker with Postgres, Qdrant, and a stub embedder.

pub async fn run_worker(state: WorkerState) -> Result<()> {
    loop {
        if let Err(err) = process_once(&state).await {
            error!(error = %err, "Outbox processing failed.");
        }
        tokio::time::sleep(to_std_duration(Duration::milliseconds(POLL_INTERVAL_MS))).await;
    }
}

async fn process_once(state: &WorkerState) -> Result<()> {
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
        }
        Err(err) => {
            mark_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
            error!(error = %err, outbox_id = %job.outbox_id, "Outbox job failed.");
        }
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
    let delete = DeletePointsBuilder::new(state.qdrant.collection.clone())
        .points([point_id])
        .wait(true);
    state.qdrant.client.delete_points(delete).await?;
    Ok(())
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
    if let Some(expires_at) = note.expires_at {
        if expires_at <= now {
            return false;
        }
    }
    true
}

async fn ensure_embedding(
    state: &WorkerState,
    note: &MemoryNote,
    embedding_version: &str,
) -> Result<Vec<f32>> {
    if let Some(embedding) = fetch_embedding(&state.db, note.note_id, embedding_version).await? {
        validate_vector_dim(&embedding, state.qdrant.vector_dim)?;
        return Ok(embedding);
    }

    let vectors = elf_providers::embedding::embed(&state.embedding, &[note.text.clone()]).await?;
    let Some(vector) = vectors.into_iter().next() else {
        return Err(eyre!("Embedding provider returned no vectors."));
    };
    validate_vector_dim(&vector, state.qdrant.vector_dim)?;
    insert_embedding(
        &state.db,
        note.note_id,
        embedding_version,
        vector.len() as i32,
        &vector,
    )
    .await?;
    Ok(vector)
}

async fn fetch_embedding(
    db: &Db,
    note_id: uuid::Uuid,
    embedding_version: &str,
) -> Result<Option<Vec<f32>>> {
    #[derive(sqlx::FromRow)]
    struct EmbeddingRow {
        embedding_dim: i32,
        vec_text: String,
    }

    let row = sqlx::query_as::<_, EmbeddingRow>(
        "SELECT embedding_dim, vec::text AS vec_text \
         FROM note_embeddings WHERE note_id = $1 AND embedding_version = $2",
    )
    .bind(note_id)
    .bind(embedding_version)
    .fetch_optional(&db.pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let vec = parse_vector_text(&row.vec_text)?;
    if vec.len() as i32 != row.embedding_dim {
        return Err(eyre!(
            "Stored embedding dim does not match vector length."
        ));
    }
    Ok(Some(vec))
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
         ON CONFLICT (note_id, embedding_version) DO NOTHING",
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
    payload_map.insert(
        "project_id".to_string(),
        Value::from(note.project_id.clone()),
    );
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
    payload_map.insert(
        "importance".to_string(),
        Value::from(JsonValue::from(note.importance as f64)),
    );
    payload_map.insert(
        "confidence".to_string(),
        Value::from(JsonValue::from(note.confidence as f64)),
    );
    payload_map.insert(
        "embedding_version".to_string(),
        Value::from(note.embedding_version.clone()),
    );

    let payload = Payload::from(payload_map);
    let point = PointStruct::new(note.note_id.to_string(), vec.to_vec(), payload);
    let upsert = UpsertPointsBuilder::new(state.qdrant.collection.clone(), vec![point]).wait(true);
    state.qdrant.client.upsert_points(upsert).await?;
    Ok(())
}

fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
    use time::format_description::well_known::Rfc3339;
    ts.format(&Rfc3339)
        .map_err(|_| eyre!("Failed to format timestamp."))
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

fn parse_vector_text(raw: &str) -> Result<Vec<f32>> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| eyre!("Vector text has invalid format."))?;
    let inner = inner.trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut values = Vec::new();
    for part in inner.split(',') {
        let value: f32 = part
            .trim()
            .parse()
            .map_err(|_| eyre!("Vector value is not a valid float."))?;
        values.push(value);
    }
    Ok(values)
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

async fn mark_done(db: &Db, outbox_id: uuid::Uuid) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    sqlx::query(
        "UPDATE indexing_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2",
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
