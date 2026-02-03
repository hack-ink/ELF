use elf_storage::models::MemoryNote;

use crate::{ElfService, ServiceError, ServiceResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: Option<String>,
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
	#[serde(with = "crate::time_serde")]
	pub updated_at: time::OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<time::OffsetDateTime>,
	pub source_ref: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListResponse {
	pub items: Vec<ListItem>,
}

impl ElfService {
	pub async fn list(&self, req: ListRequest) -> ServiceResult<ListResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(ServiceError::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}
		if let Some(agent_id) = req.agent_id.as_ref()
			&& agent_id.trim().is_empty()
		{
			return Err(ServiceError::InvalidRequest {
				message: "agent_id must not be empty when provided.".to_string(),
			});
		}
		if let Some(scope) = req.scope.as_ref()
			&& !self.cfg.scopes.allowed.iter().any(|value| value == scope)
		{
			return Err(ServiceError::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		let mut builder = sqlx::QueryBuilder::new(
			"SELECT note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at \
             FROM memory_notes WHERE tenant_id = ",
		);
		builder.push_bind(tenant_id);
		builder.push(" AND project_id = ");
		builder.push_bind(project_id);

		if let Some(scope) = &req.scope {
			builder.push(" AND scope = ");
			builder.push_bind(scope);
			if scope == "agent_private" {
				let agent_id = req.agent_id.as_ref().map(|value| value.trim()).unwrap_or("");
				if agent_id.is_empty() {
					return Err(ServiceError::ScopeDenied {
						message: "agent_id is required for agent_private scope.".to_string(),
					});
				}
				builder.push(" AND agent_id = ");
				builder.push_bind(agent_id);
			}
		} else {
			builder.push(" AND scope != ");
			builder.push_bind("agent_private");
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
