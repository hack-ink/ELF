use std::collections::HashMap;

use sqlx::{PgExecutor, Postgres, Transaction};
use uuid::Uuid;

use crate::{
	Result,
	core_blocks::{
		rows::CoreBlockEventRow,
		types::{CoreBlockAuditEvent, CoreBlockEventInput},
	},
};

pub(in crate::core_blocks) async fn fetch_audit_history<'e, E>(
	executor: E,
	block_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<CoreBlockAuditEvent>>>
where
	E: PgExecutor<'e>,
{
	if block_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows = sqlx::query_as::<_, CoreBlockEventRow>(
		"\
SELECT
	event_id,
	block_id,
	attachment_id,
	actor_agent_id,
	event_type,
	target_agent_id,
	read_profile,
	prev_snapshot,
	new_snapshot,
	reason,
	ts
FROM core_memory_block_events
WHERE block_id = ANY($1)
ORDER BY ts ASC, event_id ASC",
	)
	.bind(block_ids)
	.fetch_all(executor)
	.await?;
	let mut by_block: HashMap<Uuid, Vec<CoreBlockAuditEvent>> = HashMap::new();

	for row in rows {
		by_block.entry(row.block_id).or_default().push(CoreBlockAuditEvent {
			event_id: row.event_id,
			block_id: row.block_id,
			attachment_id: row.attachment_id,
			actor_agent_id: row.actor_agent_id,
			event_type: row.event_type,
			target_agent_id: row.target_agent_id,
			read_profile: row.read_profile,
			prev_snapshot: row.prev_snapshot,
			new_snapshot: row.new_snapshot,
			reason: row.reason,
			ts: row.ts,
		});
	}

	Ok(by_block)
}

pub(in crate::core_blocks) async fn insert_core_block_event(
	tx: &mut Transaction<'_, Postgres>,
	event: CoreBlockEventInput<'_>,
) -> Result<()> {
	sqlx::query(
		"\
INSERT INTO core_memory_block_events (
	event_id,
	block_id,
	attachment_id,
	tenant_id,
	project_id,
	actor_agent_id,
	event_type,
	target_agent_id,
	read_profile,
	prev_snapshot,
	new_snapshot,
	reason,
	ts
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
	)
	.bind(Uuid::new_v4())
	.bind(event.block_id)
	.bind(event.attachment_id)
	.bind(event.tenant_id)
	.bind(event.project_id)
	.bind(event.actor_agent_id)
	.bind(event.event_type)
	.bind(event.target_agent_id)
	.bind(event.read_profile)
	.bind(event.prev_snapshot)
	.bind(event.new_snapshot)
	.bind(event.reason)
	.bind(event.ts)
	.execute(&mut **tx)
	.await?;

	Ok(())
}
