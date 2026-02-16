use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, InsertVersionArgs, NoteOp, Result};
use elf_storage::models::MemoryNote;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub note_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteResponse {
	pub note_id: Uuid,
	pub op: NoteOp,
}

impl ElfService {
	pub async fn delete(&self, req: DeleteRequest) -> Result<DeleteResponse> {
		let now = OffsetDateTime::now_utc();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

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
		.ok_or_else(|| Error::InvalidRequest { message: "Note not found.".to_string() })?;

		if note.scope == "agent_private" && note.agent_id != agent_id {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}

		let scope_allowed = self.cfg.scopes.allowed.iter().any(|scope| scope == &note.scope);
		let write_allowed = match note.scope.as_str() {
			"agent_private" => self.cfg.scopes.write_allowed.agent_private,
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};

		if !scope_allowed || !write_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}
		if note.status == "deleted" {
			tx.commit().await?;

			return Ok(DeleteResponse { note_id: note.note_id, op: NoteOp::None });
		}

		let prev_snapshot = crate::note_snapshot(&note);

		note.status = "deleted".to_string();
		note.updated_at = now;

		sqlx::query!(
			"UPDATE memory_notes SET status = $1, updated_at = $2 WHERE note_id = $3",
			note.status.as_str(),
			note.updated_at,
			note.note_id,
		)
		.execute(&mut *tx)
		.await?;

		crate::insert_version(
			&mut *tx,
			InsertVersionArgs {
				note_id: note.note_id,
				op: "DELETE",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&note)),
				reason: "delete",
				actor: agent_id,
				ts: now,
			},
		)
		.await?;
		crate::enqueue_outbox_tx(&mut *tx, note.note_id, "DELETE", &note.embedding_version, now)
			.await?;

		tx.commit().await?;

		Ok(DeleteResponse { note_id: note.note_id, op: NoteOp::Delete })
	}
}
