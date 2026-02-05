// crates.io
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

// self
use elf_storage::models::MemoryNote;

use crate::{ElfService, ServiceError, ServiceResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteFetchRequest {
	pub note_id: Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteFetchResponse {
	pub note_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	#[serde(rename = "type")]
	pub note_type: String,
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
}

impl ElfService {
	pub async fn get_note(&self, req: NoteFetchRequest) -> ServiceResult<NoteFetchResponse> {
		let row: Option<MemoryNote> =
			sqlx::query_as("SELECT * FROM memory_notes WHERE note_id = $1")
				.bind(req.note_id)
				.fetch_optional(&self.db.pool)
				.await?;
		let Some(note) = row else {
			return Err(ServiceError::InvalidRequest { message: "Unknown note_id.".to_string() });
		};
		Ok(NoteFetchResponse {
			note_id: note.note_id,
			tenant_id: note.tenant_id,
			project_id: note.project_id,
			agent_id: note.agent_id,
			scope: note.scope,
			note_type: note.r#type,
			key: note.key,
			text: note.text,
			importance: note.importance,
			confidence: note.confidence,
			status: note.status,
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref,
		})
	}
}
