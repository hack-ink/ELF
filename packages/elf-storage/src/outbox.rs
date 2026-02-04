use color_eyre::Result;
use uuid::Uuid;

use crate::db::Db;

pub async fn enqueue_outbox(
	db: &Db,
	note_id: Uuid,
	op: &str,
	embedding_version: &str,
) -> Result<()> {
	sqlx::query(
        "INSERT INTO indexing_outbox (outbox_id, note_id, op, embedding_version, status) VALUES ($1,$2,$3,$4,'PENDING')",
    )
    .bind(Uuid::new_v4())
    .bind(note_id)
    .bind(op)
    .bind(embedding_version)
    .execute(&db.pool)
    .await?;
	Ok(())
}
