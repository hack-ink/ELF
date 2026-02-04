use std::collections::HashMap;

use qdrant_client::{
	client::Payload,
	qdrant::{Document, PointStruct, UpsertPointsBuilder, Vector},
};

use crate::{ElfService, ServiceError, ServiceResult, parse_pg_vector};
use elf_storage::qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RebuildReport {
	pub rebuilt_count: u64,
	pub missing_vector_count: u64,
	pub error_count: u64,
}

#[derive(sqlx::FromRow)]
struct RebuildRow {
	note_id: uuid::Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	scope: String,
	#[sqlx(rename = "type")]
	note_type: String,
	key: Option<String>,
	text: String,
	status: String,
	updated_at: time::OffsetDateTime,
	expires_at: Option<time::OffsetDateTime>,
	importance: f32,
	confidence: f32,
	embedding_version: String,
	vec_text: Option<String>,
}

impl ElfService {
	pub async fn rebuild_qdrant(&self) -> ServiceResult<RebuildReport> {
		let now = time::OffsetDateTime::now_utc();
		let rows: Vec<RebuildRow> = sqlx::query_as(
            "SELECT n.note_id, n.tenant_id, n.project_id, n.agent_id, n.scope, n.type, n.key, n.text, n.status, \
             n.updated_at, n.expires_at, n.importance, n.confidence, n.embedding_version, \
             e.vec::text AS vec_text \
             FROM memory_notes n \
             LEFT JOIN note_embeddings e \
             ON n.note_id = e.note_id AND n.embedding_version = e.embedding_version \
             WHERE n.status = 'active' AND (n.expires_at IS NULL OR n.expires_at > $1)",
        )
        .bind(now)
        .fetch_all(&self.db.pool)
        .await?;

		let mut rebuilt_count = 0u64;
		let mut missing_vector_count = 0u64;
		let mut error_count = 0u64;

		for row in rows {
			let Some(vec_text) = row.vec_text else {
				missing_vector_count += 1;
				continue;
			};
			let vec = match parse_pg_vector(&vec_text) {
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
			payload.insert("tenant_id", row.tenant_id);
			payload.insert("project_id", row.project_id);
			payload.insert("agent_id", row.agent_id);
			payload.insert("scope", row.scope);
			payload.insert("type", row.note_type);
			payload.insert(
				"key",
				row.key.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
			);
			payload.insert("status", row.status);
			payload
				.insert("updated_at", serde_json::Value::String(format_timestamp(row.updated_at)?));
			let expires_value = match row.expires_at {
				Some(ts) => serde_json::Value::String(format_timestamp(ts)?),
				None => serde_json::Value::Null,
			};
			payload.insert("expires_at", expires_value);
			payload.insert("importance", serde_json::Value::from(row.importance as f64));
			payload.insert("confidence", serde_json::Value::from(row.confidence as f64));
			payload.insert("embedding_version", row.embedding_version.clone());

			let mut vectors = HashMap::new();
			vectors.insert(DENSE_VECTOR_NAME.to_string(), Vector::from(vec));
			vectors.insert(
				BM25_VECTOR_NAME.to_string(),
				Vector::from(Document::new(row.text, BM25_MODEL)),
			);
			let point = PointStruct::new(row.note_id.to_string(), vectors, payload);
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

fn format_timestamp(ts: time::OffsetDateTime) -> ServiceResult<String> {
	use time::format_description::well_known::Rfc3339;
	ts.format(&Rfc3339).map_err(|_| ServiceError::InvalidRequest {
		message: "Failed to format timestamp.".to_string(),
	})
}
