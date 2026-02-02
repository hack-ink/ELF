use color_eyre::Result;

use crate::{db::Db, models::MemoryNote};

pub async fn insert_note(db: &Db, note: &MemoryNote) -> Result<()> {
	sqlx::query(
        "INSERT INTO memory_notes (note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at)\
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
    )
    .bind(note.note_id)
    .bind(&note.tenant_id)
    .bind(&note.project_id)
    .bind(&note.agent_id)
    .bind(&note.scope)
    .bind(&note.r#type)
    .bind(&note.key)
    .bind(&note.text)
    .bind(note.importance)
    .bind(note.confidence)
    .bind(&note.status)
    .bind(note.created_at)
    .bind(note.updated_at)
    .bind(note.expires_at)
    .bind(&note.embedding_version)
    .bind(&note.source_ref)
    .bind(note.hit_count)
    .bind(note.last_hit_at)
    .execute(&db.pool)
    .await?;
	Ok(())
}

pub async fn update_note(db: &Db, note: &MemoryNote) -> Result<()> {
	sqlx::query(
        "UPDATE memory_notes SET text = $1, importance = $2, confidence = $3, updated_at = $4, expires_at = $5, source_ref = $6 WHERE note_id = $7",
    )
    .bind(&note.text)
    .bind(note.importance)
    .bind(note.confidence)
    .bind(note.updated_at)
    .bind(note.expires_at)
    .bind(&note.source_ref)
    .bind(note.note_id)
    .execute(&db.pool)
    .await?;
	Ok(())
}
