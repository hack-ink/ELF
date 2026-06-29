use crate::{
	ElfService, Error, InsertVersionArgs,
	access::{self, ORG_PROJECT_ID},
};
use elf_storage::models::MemoryNote;

use super::types::{
	PublishNoteRequest, PublishNoteResponse, UnpublishNoteRequest, UnpublishNoteResponse,
};

impl ElfService {
	/// Publishes an owned note into a shared scope.
	pub async fn publish_note(
		&self,
		req: PublishNoteRequest,
	) -> crate::Result<PublishNoteResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let mut tx = self.db.pool.begin().await?;
		let mut note: MemoryNote = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
	AND tenant_id = $2
	AND project_id IN ($3, $4)
FOR UPDATE",
		)
		.bind(req.note_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ORG_PROJECT_ID)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| Error::InvalidRequest { message: "Note not found.".to_string() })?;

		if note.agent_id != agent_id {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.status != "active" {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.expires_at.map(|ts| ts <= time::OffsetDateTime::now_utc()).unwrap_or(false) {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}

		let scope = req.scope.as_str();
		let scope_allowed = match scope {
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};

		if !scope_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		let target_project_id = if scope == "org_shared" { ORG_PROJECT_ID } else { project_id };

		access::ensure_active_project_scope_grant(
			&mut *tx,
			tenant_id,
			target_project_id,
			scope,
			agent_id,
		)
		.await?;

		if note.scope == scope && note.project_id == target_project_id {
			return Ok(PublishNoteResponse { note_id: note.note_id, scope: note.scope });
		}

		let now = time::OffsetDateTime::now_utc();
		let prev_snapshot = crate::note_snapshot(&note);

		note.scope = scope.to_string();
		note.project_id = target_project_id.to_string();
		note.updated_at = now;

		crate::insert_version(
			&mut *tx,
			InsertVersionArgs {
				note_id: note.note_id,
				op: "PUBLISH",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&note)),
				reason: "publish_note",
				actor: agent_id,
				ts: now,
			},
		)
		.await?;
		sqlx::query(
			"UPDATE memory_notes SET scope = $1, project_id = $2, updated_at = $3 WHERE note_id = $4",
		)
		.bind(scope)
		.bind(note.project_id.as_str())
		.bind(now)
		.bind(note.note_id)
		.execute(&mut *tx)
		.await?;
		crate::enqueue_outbox_tx(&mut *tx, note.note_id, "UPSERT", &note.embedding_version, now)
			.await?;

		tx.commit().await?;

		Ok(PublishNoteResponse { note_id: note.note_id, scope: note.scope })
	}

	/// Returns a previously published note to its non-shared scope.
	pub async fn unpublish_note(
		&self,
		req: UnpublishNoteRequest,
	) -> crate::Result<UnpublishNoteResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let mut tx = self.db.pool.begin().await?;
		let mut note: MemoryNote = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
	AND tenant_id = $2
	AND project_id IN ($3, $4)
FOR UPDATE",
		)
		.bind(req.note_id)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ORG_PROJECT_ID)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| Error::InvalidRequest { message: "Note not found.".to_string() })?;

		if note.agent_id != agent_id {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.status != "active" {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if note.expires_at.map(|ts| ts <= time::OffsetDateTime::now_utc()).unwrap_or(false) {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		}
		if !self.cfg.scopes.write_allowed.agent_private {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}
		if note.scope == "agent_private" {
			return Ok(UnpublishNoteResponse { note_id: note.note_id, scope: note.scope });
		}

		let now = time::OffsetDateTime::now_utc();
		let prev_snapshot = crate::note_snapshot(&note);

		if note.scope == "org_shared" && note.project_id == ORG_PROJECT_ID {
			note.project_id = project_id.to_string();
		}

		note.scope = "agent_private".to_string();
		note.updated_at = now;

		crate::insert_version(
			&mut *tx,
			InsertVersionArgs {
				note_id: note.note_id,
				op: "UNPUBLISH",
				prev_snapshot: Some(prev_snapshot),
				new_snapshot: Some(crate::note_snapshot(&note)),
				reason: "unpublish_note",
				actor: agent_id,
				ts: now,
			},
		)
		.await?;
		sqlx::query(
			"UPDATE memory_notes SET scope = $1, project_id = $2, updated_at = $3 WHERE note_id = $4",
		)
		.bind(note.scope.as_str())
		.bind(note.project_id.as_str())
		.bind(now)
		.bind(note.note_id)
		.execute(&mut *tx)
		.await?;
		crate::enqueue_outbox_tx(&mut *tx, note.note_id, "UPSERT", &note.embedding_version, now)
			.await?;

		tx.commit().await?;

		Ok(UnpublishNoteResponse { note_id: note.note_id, scope: note.scope })
	}
}
