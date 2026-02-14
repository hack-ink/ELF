use std::collections::HashMap;

use qdrant_client::{
	client::Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RebuildReport {
	pub rebuilt_count: u64,
	pub missing_vector_count: u64,
	pub error_count: u64,
}

#[derive(sqlx::FromRow)]
struct RebuildRow {
	chunk_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	chunk_text: String,
	note_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	scope: String,
	#[sqlx(rename = "type")]
	note_type: String,
	key: Option<String>,
	status: String,
	updated_at: OffsetDateTime,
	expires_at: Option<OffsetDateTime>,
	importance: f32,
	confidence: f32,
	embedding_version: String,
	vec_text: Option<String>,
}

impl ElfService {
	pub async fn rebuild_qdrant(&self) -> Result<RebuildReport> {
		let now = OffsetDateTime::now_utc();
		let rows: Vec<RebuildRow> = sqlx::query_as!(
			RebuildRow,
			"\
SELECT
	c.chunk_id,
	c.chunk_index,
	c.start_offset,
	c.end_offset,
	c.text AS chunk_text,
	n.note_id,
	n.tenant_id,
	n.project_id,
	n.agent_id,
	n.scope,
	n.type AS note_type,
	n.key,
	n.status,
	n.updated_at,
	n.expires_at,
	n.importance,
	n.confidence,
	c.embedding_version,
	e.vec::text AS \"vec_text?\"
FROM memory_note_chunks c
JOIN memory_notes n ON n.note_id = c.note_id
LEFT JOIN note_chunk_embeddings e
	ON e.chunk_id = c.chunk_id AND e.embedding_version = c.embedding_version
WHERE n.status = 'active' AND (n.expires_at IS NULL OR n.expires_at > $1)",
			now,
		)
		.fetch_all(&self.db.pool)
		.await?;
		let mut rebuilt_count = 0_u64;
		let mut missing_vector_count = 0_u64;
		let mut error_count = 0_u64;

		for row in rows {
			let Some(vec_text) = row.vec_text else {
				missing_vector_count += 1;

				continue;
			};
			let vec = match crate::parse_pg_vector(&vec_text) {
				Ok(vec) => vec,
				Err(_) => {
					error_count += 1;

					continue;
				},
			};

			if vec.len() != self.cfg.storage.qdrant.vector_dim as usize {
				error_count += 1;

				continue;
			}

			let mut payload = Payload::new();

			payload.insert("note_id", row.note_id.to_string());
			payload.insert("chunk_id", row.chunk_id.to_string());
			payload.insert("chunk_index", Value::from(row.chunk_index));
			payload.insert("start_offset", Value::from(row.start_offset));
			payload.insert("end_offset", Value::from(row.end_offset));
			payload.insert("tenant_id", row.tenant_id);
			payload.insert("project_id", row.project_id);
			payload.insert("agent_id", row.agent_id);
			payload.insert("scope", row.scope);
			payload.insert("type", row.note_type);
			payload.insert("key", row.key.map(Value::String).unwrap_or(Value::Null));
			payload.insert("status", row.status);
			payload.insert("updated_at", Value::String(format_timestamp(row.updated_at)?));

			let expires_value = match row.expires_at {
				Some(ts) => Value::String(format_timestamp(ts)?),
				None => Value::Null,
			};

			payload.insert("expires_at", expires_value);
			payload.insert("importance", Value::from(row.importance as f64));
			payload.insert("confidence", Value::from(row.confidence as f64));
			payload.insert("embedding_version", row.embedding_version.clone());

			let mut vectors = HashMap::new();

			vectors.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec));
			vectors.insert(
				BM25_VECTOR_NAME.to_string(),
				Vector::from(Document::new(row.chunk_text, BM25_MODEL)),
			);

			let point = PointStruct::new(row.chunk_id.to_string(), vectors, payload);
			let result = self
				.qdrant
				.client
				.upsert_points(
					UpsertPointsBuilder::new(self.qdrant.collection.clone(), vec![point])
						.wait(true),
				)
				.await;

			if result.is_err() {
				error_count += 1;

				continue;
			}

			rebuilt_count += 1;
		}

		Ok(RebuildReport { rebuilt_count, missing_vector_count, error_count })
	}
}

fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	ts.format(&Rfc3339)
		.map_err(|_| Error::InvalidRequest { message: "Failed to format timestamp.".to_string() })
}
