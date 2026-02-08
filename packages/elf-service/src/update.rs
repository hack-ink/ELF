use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, InsertVersionArgs, NoteOp, ServiceError, ServiceResult};
use elf_domain::{cjk, ttl, writegate};
use elf_storage::models::MemoryNote;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub note_id: Uuid,
	pub text: Option<String>,
	pub importance: Option<f32>,
	pub confidence: Option<f32>,
	pub ttl_days: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateResponse {
	pub note_id: Uuid,
	pub op: NoteOp,
	pub reason_code: Option<String>,
}

impl ElfService {
	pub async fn update(&self, req: UpdateRequest) -> ServiceResult<UpdateResponse> {
		let now = OffsetDateTime::now_utc();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(ServiceError::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		if req.text.is_none()
			&& req.importance.is_none()
			&& req.confidence.is_none()
			&& req.ttl_days.is_none()
		{
			return Err(ServiceError::InvalidRequest {
				message: "No updates provided.".to_string(),
			});
		}

		let text_update = req.text.clone();
		let mut tx = self.db.pool.begin().await?;
		let mut note: MemoryNote = sqlx::query_as!(
			MemoryNote,
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1 AND tenant_id = $2 AND project_id = $3
FOR UPDATE",
			req.note_id,
			tenant_id,
			project_id,
		)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| ServiceError::InvalidRequest { message: "Note not found.".to_string() })?;

		if note.scope == "agent_private" && note.agent_id != agent_id {
			return Err(ServiceError::InvalidRequest { message: "Note not found.".to_string() });
		}
		if !note.status.eq_ignore_ascii_case("active") {
			return Err(ServiceError::InvalidRequest { message: "Note not found.".to_string() });
		}

		if let Some(expires_at) = note.expires_at
			&& expires_at <= now
		{
			return Err(ServiceError::InvalidRequest { message: "Note not found.".to_string() });
		}

		let prev_snapshot = crate::note_snapshot(&note);
		let candidate_text = if let Some(text) = text_update.as_ref() {
			if cjk::contains_cjk(text) {
				return Err(ServiceError::NonEnglishInput { field: "$.text".to_string() });
			}
			text.clone()
		} else {
			note.text.clone()
		};
		let gate = writegate::NoteInput {
			note_type: note.r#type.clone(),
			scope: note.scope.clone(),
			text: candidate_text,
		};

		if let Err(code) = writegate::writegate(&gate, &self.cfg) {
			return Ok(UpdateResponse {
				note_id: note.note_id,
				op: NoteOp::Rejected,
				reason_code: Some(crate::writegate_reason_code(code).to_string()),
			});
		}

		let next_text = text_update.unwrap_or_else(|| note.text.clone());
		let next_importance = req.importance.unwrap_or(note.importance);
		let next_confidence = req.confidence.unwrap_or(note.confidence);
		let next_expires_at = match req.ttl_days {
			Some(ttl_days) => ttl::compute_expires_at(Some(ttl_days), &note.r#type, &self.cfg, now),
			None => note.expires_at,
		};
		let changed = next_text != note.text
			|| (next_importance - note.importance).abs() > f32::EPSILON
			|| (next_confidence - note.confidence).abs() > f32::EPSILON
			|| next_expires_at != note.expires_at;

		if !changed {
			tx.commit().await?;

			return Ok(UpdateResponse {
				note_id: note.note_id,
				op: NoteOp::None,
				reason_code: None,
			});
		}

		note.text = next_text;
		note.importance = next_importance;
		note.confidence = next_confidence;
		note.expires_at = next_expires_at;
		note.updated_at = now;

		sqlx::query!(
			"\
UPDATE memory_notes
SET
	text = $1,
	importance = $2,
	confidence = $3,
	updated_at = $4,
	expires_at = $5
WHERE note_id = $6",
			note.text.as_str(),
			note.importance,
			note.confidence,
			note.updated_at,
			note.expires_at,
			note.note_id,
		)
		.execute(&mut *tx)
		.await?;
		crate::insert_version(
			&mut tx,
			InsertVersionArgs {
				note_id: note.note_id,
				op: "UPDATE",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&note)),
				reason: "update",
				actor: "update",
				ts: note.updated_at,
			},
		)
		.await?;
		crate::enqueue_outbox_tx(
			&mut tx,
			note.note_id,
			"UPSERT",
			&note.embedding_version,
			note.updated_at,
		)
		.await?;

		tx.commit().await?;

		Ok(UpdateResponse { note_id: note.note_id, op: NoteOp::Update, reason_code: None })
	}
}
