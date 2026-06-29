use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use elf_storage::models::MemoryNote;

pub(crate) struct InsertVersionArgs<'a> {
	pub(crate) note_id: Uuid,
	pub(crate) op: &'a str,
	pub(crate) prev_snapshot: Option<Value>,
	pub(crate) new_snapshot: Option<Value>,
	pub(crate) reason: &'a str,
	pub(crate) actor: &'a str,
	pub(crate) ts: OffsetDateTime,
}

pub(crate) fn note_snapshot(note: &MemoryNote) -> Value {
	serde_json::json!({
		"note_id": note.note_id,
		"tenant_id": note.tenant_id,
		"project_id": note.project_id,
		"agent_id": note.agent_id,
		"scope": note.scope,
		"type": note.r#type,
		"key": note.key,
		"text": note.text,
		"importance": note.importance,
		"confidence": note.confidence,
		"status": note.status,
		"created_at": note.created_at,
		"updated_at": note.updated_at,
		"expires_at": note.expires_at,
		"embedding_version": note.embedding_version,
		"source_ref": note.source_ref,
		"hit_count": note.hit_count,
		"last_hit_at": note.last_hit_at,
	})
}

pub(crate) async fn insert_version<'e, E>(executor: E, args: InsertVersionArgs<'_>) -> Result<Uuid>
where
	E: PgExecutor<'e>,
{
	let InsertVersionArgs { note_id, op, prev_snapshot, new_snapshot, reason, actor, ts } = args;
	let version_id = Uuid::new_v4();

	sqlx::query(
		"\
INSERT INTO memory_note_versions (
	version_id,
	note_id,
	op,
	prev_snapshot,
	new_snapshot,
	reason,
	actor,
	ts
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
	)
	.bind(version_id)
	.bind(note_id)
	.bind(op)
	.bind(prev_snapshot)
	.bind(new_snapshot)
	.bind(reason)
	.bind(actor)
	.bind(ts)
	.execute(executor)
	.await?;

	Ok(version_id)
}

pub(crate) async fn enqueue_outbox_tx<'e, E>(
	executor: E,
	note_id: Uuid,
	op: &str,
	embedding_version: &str,
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO indexing_outbox (
	outbox_id,
	note_id,
	op,
	embedding_version,
	status,
	created_at,
	updated_at,
	available_at
)
VALUES ($1,$2,$3,$4,'PENDING',$5,$6,$7)",
	)
	.bind(Uuid::new_v4())
	.bind(note_id)
	.bind(op)
	.bind(embedding_version)
	.bind(now)
	.bind(now)
	.bind(now)
	.execute(executor)
	.await?;

	Ok(())
}
