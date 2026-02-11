use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::QueryBuilder;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_storage::models::MemoryNote;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: Option<String>,
	pub scope: Option<String>,
	pub status: Option<String>,
	pub r#type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListItem {
	pub note_id: Uuid,
	pub r#type: String,
	pub key: Option<String>,
	pub scope: String,
	pub status: String,
	pub text: String,
	pub importance: f32,
	pub confidence: f32,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<OffsetDateTime>,
	pub source_ref: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListResponse {
	pub items: Vec<ListItem>,
}

impl ElfService {
	pub async fn list(&self, req: ListRequest) -> Result<ListResponse> {
		let now = OffsetDateTime::now_utc();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}

		if let Some(agent_id) = req.agent_id.as_ref()
			&& agent_id.trim().is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "agent_id must not be empty when provided.".to_string(),
			});
		}
		if let Some(scope) = req.scope.as_ref()
			&& !self.cfg.scopes.allowed.iter().any(|value| value == scope)
		{
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		let mut builder = QueryBuilder::new(
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
					return Err(Error::ScopeDenied {
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

		let requested_status = req.status.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());

		if let Some(status) = requested_status {
			builder.push(" AND status = ");
			builder.push_bind(status);
		} else {
			builder.push(" AND status = ");
			builder.push_bind("active");
		}

		// Expiry only applies to active notes. Deleted notes may also have expires_at set by GC.
		if requested_status.unwrap_or("active").eq_ignore_ascii_case("active") {
			builder.push(" AND (expires_at IS NULL OR expires_at > ");
			builder.push_bind(now);
			builder.push(")");
		}

		if let Some(note_type) = &req.r#type {
			builder.push(" AND type = ");
			builder.push_bind(note_type);
		}

		let notes: Vec<MemoryNote> = builder.build_query_as().fetch_all(&self.db.pool).await?;
		let items = notes
			.into_iter()
			.map(|note| ListItem {
				note_id: note.note_id,
				r#type: note.r#type,
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
