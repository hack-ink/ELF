use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result, access, structured_fields::StructuredFields};
use elf_storage::models::MemoryNote;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteFetchRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub note_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteFetchResponse {
	pub note_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub r#type: String,
	pub key: Option<String>,
	pub text: String,
	pub importance: f32,
	pub confidence: f32,
	pub status: String,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<OffsetDateTime>,
	pub source_ref: Value,
	pub structured: Option<StructuredFields>,
}

impl ElfService {
	pub async fn get_note(&self, req: NoteFetchRequest) -> Result<NoteFetchResponse> {
		let now = OffsetDateTime::now_utc();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let allowed_scopes = self.cfg.scopes.allowed.clone();
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let row: Option<MemoryNote> = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )",
		)
		.bind(req.note_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(access::ORG_PROJECT_ID)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(note) = row else {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		};
		let shared_grants = if note.scope == "agent_private" {
			HashSet::new()
		} else {
			access::load_shared_read_grants_with_org_shared(
				&self.db.pool,
				tenant_id,
				project_id,
				agent_id,
				org_shared_allowed,
			)
			.await?
		};

		if !access::note_read_allowed(&note, agent_id, &allowed_scopes, &shared_grants, now) {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}

		let structured = crate::structured_fields::fetch_structured_fields(
			&self.db.pool,
			std::slice::from_ref(&note.note_id),
		)
		.await?
		.remove(&note.note_id);

		Ok(NoteFetchResponse {
			note_id: note.note_id,
			tenant_id: note.tenant_id,
			project_id: note.project_id,
			agent_id: note.agent_id,
			scope: note.scope,
			r#type: note.r#type,
			key: note.key,
			text: note.text,
			importance: note.importance,
			confidence: note.confidence,
			status: note.status,
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref,
			structured,
		})
	}
}
