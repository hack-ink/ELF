use elf_storage::models::MemoryNote;

use crate::{ElfService, NoteOp, ServiceError, ServiceResult, enqueue_outbox_tx, insert_version, note_snapshot};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteRequest {
    pub tenant_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub note_id: uuid::Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteResponse {
    pub note_id: uuid::Uuid,
    pub op: NoteOp,
}

impl ElfService {
    pub async fn delete(&self, req: DeleteRequest) -> ServiceResult<DeleteResponse> {
        let now = time::OffsetDateTime::now_utc();
        let tenant_id = req.tenant_id.trim();
        let project_id = req.project_id.trim();
        let agent_id = req.agent_id.trim();
        if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty()
        {
            return Err(ServiceError::InvalidRequest {
                message: "tenant_id, project_id, and agent_id are required.".to_string(),
            });
        }
        let mut tx = self.db.pool.begin().await?;
        let mut note: MemoryNote = sqlx::query_as(
            "SELECT * FROM memory_notes \
             WHERE note_id = $1 AND tenant_id = $2 AND project_id = $3 AND agent_id = $4 \
             FOR UPDATE",
        )
        .bind(req.note_id)
        .bind(tenant_id)
        .bind(project_id)
        .bind(agent_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ServiceError::InvalidRequest {
            message: "Note not found.".to_string(),
        })?;

        let scope_allowed = self
            .cfg
            .scopes
            .allowed
            .iter()
            .any(|scope| scope == &note.scope);
        let write_allowed = match note.scope.as_str() {
            "agent_private" => self.cfg.scopes.write_allowed.agent_private,
            "project_shared" => self.cfg.scopes.write_allowed.project_shared,
            "org_shared" => self.cfg.scopes.write_allowed.org_shared,
            _ => false,
        };
        if !scope_allowed || !write_allowed {
            return Err(ServiceError::ScopeDenied {
                message: "Scope is not allowed.".to_string(),
            });
        }

        if note.status == "deleted" {
            tx.commit().await?;
            return Ok(DeleteResponse {
                note_id: note.note_id,
                op: NoteOp::None,
            });
        }

        let prev_snapshot = note_snapshot(&note);
        note.status = "deleted".to_string();
        note.updated_at = now;

        sqlx::query("UPDATE memory_notes SET status = $1, updated_at = $2 WHERE note_id = $3")
            .bind(&note.status)
            .bind(note.updated_at)
            .bind(note.note_id)
            .execute(&mut *tx)
            .await?;

        insert_version(
            &mut tx,
            note.note_id,
            "DELETE",
            Some(prev_snapshot),
            Some(note_snapshot(&note)),
            "delete",
            "delete",
            now,
        )
        .await?;
        enqueue_outbox_tx(
            &mut tx,
            note.note_id,
            "DELETE",
            &note.embedding_version,
            now,
        )
        .await?;

        tx.commit().await?;

        Ok(DeleteResponse {
            note_id: note.note_id,
            op: NoteOp::Delete,
        })
    }
}
