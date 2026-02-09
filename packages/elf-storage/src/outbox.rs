use sqlx::PgExecutor;
use uuid::Uuid;

use crate::Result;

pub async fn enqueue_outbox<'e, E>(
	executor: E,
	note_id: Uuid,
	op: &str,
	embedding_version: &str,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query!(
		"INSERT INTO indexing_outbox (outbox_id, note_id, op, embedding_version, status) \
VALUES ($1,$2,$3,$4,'PENDING')",
		Uuid::new_v4(),
		note_id,
		op,
		embedding_version,
	)
	.execute(executor)
	.await?;

	Ok(())
}
