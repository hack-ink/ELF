use elf_storage::models::MemoryNote;

use crate::{ElfService, ServiceError, ServiceResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListRequest {
    pub tenant_id: String,
    pub project_id: String,
    pub scope: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub note_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListItem {
    pub note_id: uuid::Uuid,
    #[serde(rename = "type")]
    pub note_type: String,
    pub key: Option<String>,
    pub scope: String,
    pub status: String,
    pub text: String,
    pub importance: f32,
    pub confidence: f32,
    pub updated_at: time::OffsetDateTime,
    pub expires_at: Option<time::OffsetDateTime>,
    pub source_ref: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListResponse {
    pub items: Vec<ListItem>,
}

impl ElfService {
    pub async fn list(&self, req: ListRequest) -> ServiceResult<ListResponse> {
        if req.tenant_id.trim().is_empty() || req.project_id.trim().is_empty() {
            return Err(ServiceError::InvalidRequest {
                message: "tenant_id and project_id are required.".to_string(),
            });
        }

        let mut builder = sqlx::QueryBuilder::new(
            "SELECT note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at \
             FROM memory_notes WHERE tenant_id = ",
        );
        builder.push_bind(&req.tenant_id);
        builder.push(" AND project_id = ");
        builder.push_bind(&req.project_id);

        if let Some(scope) = &req.scope {
            builder.push(" AND scope = ");
            builder.push_bind(scope);
        }
        if let Some(status) = &req.status {
            builder.push(" AND status = ");
            builder.push_bind(status);
        }
        if let Some(note_type) = &req.note_type {
            builder.push(" AND type = ");
            builder.push_bind(note_type);
        }

        let notes: Vec<MemoryNote> = builder.build_query_as().fetch_all(&self.db.pool).await?;

        let items = notes
            .into_iter()
            .map(|note| ListItem {
                note_id: note.note_id,
                note_type: note.r#type,
                key: note.key,
                scope: note.scope,
                status: note.status,
                text: note.text,
                importance: note.importance,
                confidence: note.confidence,
                updated_at: note.updated_at,
                expires_at: note.expires_at,
                source_ref: note.source_ref,
            })
            .collect();

        Ok(ListResponse { items })
    }
}
