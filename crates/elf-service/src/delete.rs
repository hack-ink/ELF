use elf_storage::models::MemoryNote;

use crate::{ElfService, NoteOp, ServiceError, ServiceResult, enqueue_outbox_tx, insert_version, note_snapshot};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteRequest {
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
        let mut tx = self.db.pool.begin().await?;
        let mut note: MemoryNote = sqlx::query_as(
            "SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
        )
        .bind(req.note_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ServiceError::InvalidRequest {
            message: "Note not found.".to_string(),
        })?;

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
