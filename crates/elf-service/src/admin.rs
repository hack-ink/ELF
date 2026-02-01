use qdrant_client::client::Payload;
use qdrant_client::qdrant::{PointStruct, UpsertPointsBuilder};

use crate::{ElfService, ServiceResult, parse_pg_vector};

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
    scope: String,
    status: String,
    embedding_version: String,
    vec_text: Option<String>,
}

impl ElfService {
    pub async fn rebuild_qdrant(&self) -> ServiceResult<RebuildReport> {
        let now = time::OffsetDateTime::now_utc();
        let rows: Vec<RebuildRow> = sqlx::query_as(
            "SELECT n.note_id, n.tenant_id, n.project_id, n.scope, n.status, n.embedding_version, \
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
                }
            };
            if vec.len() != self.cfg.storage.qdrant.vector_dim as usize {
                error_count += 1;
                continue;
            }

            let mut payload = Payload::new();
            payload.insert("tenant_id", row.tenant_id);
            payload.insert("project_id", row.project_id);
            payload.insert("scope", row.scope);
            payload.insert("status", row.status);

            let point = PointStruct::new(row.note_id.to_string(), vec, payload);
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

        Ok(RebuildReport {
            rebuilt_count,
            missing_vector_count,
            error_count,
        })
    }
}
