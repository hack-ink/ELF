use std::collections::HashMap;

use serde_json::Value;
use sqlx::{PgExecutor, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
	types::{
		CoreBlockAttachmentRow, CoreBlockAuditEvent, CoreBlockEventInput, CoreBlockEventRow,
		CoreBlockJoinedRow, CoreBlockRow, PreparedAttachRequest, PreparedDetachRequest,
		PreparedUpsertRequest,
	},
	validation::block_snapshot,
};
use crate::{Error, Result, access::ORG_PROJECT_ID};

pub(super) async fn insert_core_block(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	now: OffsetDateTime,
) -> Result<CoreBlockRow> {
	ensure_no_active_key_conflict(tx, req, None).await?;

	sqlx::query_as::<_, CoreBlockRow>(
		"\
INSERT INTO core_memory_blocks (
	block_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	key,
	title,
	content,
	source_ref,
	status,
	created_at,
	updated_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10, $10)
RETURNING *",
	)
	.bind(Uuid::new_v4())
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.bind(req.key.as_str())
	.bind(req.title.as_str())
	.bind(req.content.as_str())
	.bind(&req.source_ref)
	.bind(now)
	.fetch_one(&mut **tx)
	.await
	.map_err(Into::into)
}

pub(super) async fn update_core_block(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	block_id: Uuid,
	now: OffsetDateTime,
) -> Result<(CoreBlockRow, Option<Value>)> {
	let prev = fetch_owned_block_for_update(tx, req, block_id).await?;
	let prev_snapshot = Some(block_snapshot(&prev));

	ensure_no_active_key_conflict(tx, req, Some(block_id)).await?;

	let row = sqlx::query_as::<_, CoreBlockRow>(
		"\
UPDATE core_memory_blocks
SET
	key = $6,
	title = $7,
	content = $8,
	source_ref = $9,
	updated_at = $10
WHERE block_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND agent_id = $4
	AND scope = $5
	AND status = 'active'
RETURNING *",
	)
	.bind(block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.bind(req.key.as_str())
	.bind(req.title.as_str())
	.bind(req.content.as_str())
	.bind(&req.source_ref)
	.bind(now)
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::NotFound { message: "Core block not found.".to_string() })?;

	Ok((row, prev_snapshot))
}

async fn fetch_owned_block_for_update(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	block_id: Uuid,
) -> Result<CoreBlockRow> {
	sqlx::query_as::<_, CoreBlockRow>(
		"\
SELECT *
FROM core_memory_blocks
WHERE block_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND agent_id = $4
	AND scope = $5
	AND status = 'active'
FOR UPDATE",
	)
	.bind(block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::NotFound { message: "Core block not found.".to_string() })
}

async fn ensure_no_active_key_conflict(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	block_id: Option<Uuid>,
) -> Result<()> {
	let conflict: Option<Uuid> = sqlx::query_scalar(
		"\
SELECT block_id
FROM core_memory_blocks
WHERE tenant_id = $1
	AND project_id = $2
	AND agent_id = $3
	AND scope = $4
	AND key = $5
	AND status = 'active'
	AND ($6::uuid IS NULL OR block_id <> $6)
LIMIT 1",
	)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(req.scope.as_str())
	.bind(req.key.as_str())
	.bind(block_id)
	.fetch_optional(&mut **tx)
	.await?;

	if conflict.is_some() {
		return Err(Error::Conflict { message: "Core block key already exists.".to_string() });
	}

	Ok(())
}

pub(super) async fn fetch_active_block_for_attachment(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedAttachRequest,
) -> Result<CoreBlockRow> {
	sqlx::query_as::<_, CoreBlockRow>(
		"\
SELECT *
FROM core_memory_blocks
WHERE block_id = $1
	AND tenant_id = $2
	AND status = 'active'
	AND (
		project_id = $3
		OR (project_id = $4 AND scope = 'org_shared')
	)",
	)
	.bind(req.block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(ORG_PROJECT_ID)
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::NotFound { message: "Core block not found.".to_string() })
}

pub(super) async fn upsert_core_block_attachment(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedAttachRequest,
	now: OffsetDateTime,
) -> Result<CoreBlockAttachmentRow> {
	sqlx::query_as::<_, CoreBlockAttachmentRow>(
		"\
INSERT INTO core_memory_block_attachments (
	attachment_id,
	block_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	attached_by_agent_id,
	attached_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (tenant_id, project_id, agent_id, read_profile, block_id)
WHERE detached_at IS NULL
DO UPDATE
SET
	attached_by_agent_id = EXCLUDED.attached_by_agent_id,
	attached_at = EXCLUDED.attached_at,
	detached_by_agent_id = NULL,
	detached_at = NULL
RETURNING *",
	)
	.bind(Uuid::new_v4())
	.bind(req.block_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.target_agent_id.as_str())
	.bind(req.read_profile.as_str())
	.bind(req.agent_id.as_str())
	.bind(now)
	.fetch_one(&mut **tx)
	.await
	.map_err(Into::into)
}

pub(super) async fn fetch_active_attachment_for_update(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedDetachRequest,
) -> Result<Option<CoreBlockAttachmentRow>> {
	sqlx::query_as::<_, CoreBlockAttachmentRow>(
		"\
SELECT *
FROM core_memory_block_attachments
WHERE attachment_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND detached_at IS NULL
FOR UPDATE",
	)
	.bind(req.attachment_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.fetch_optional(&mut **tx)
	.await
	.map_err(Into::into)
}

pub(super) async fn detach_core_block_attachment(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedDetachRequest,
	now: OffsetDateTime,
) -> Result<CoreBlockAttachmentRow> {
	sqlx::query_as::<_, CoreBlockAttachmentRow>(
		"\
UPDATE core_memory_block_attachments
SET
	detached_by_agent_id = $4,
	detached_at = $5
WHERE attachment_id = $1
	AND tenant_id = $2
	AND project_id = $3
	AND detached_at IS NULL
RETURNING *",
	)
	.bind(req.attachment_id)
	.bind(req.tenant_id.as_str())
	.bind(req.project_id.as_str())
	.bind(req.agent_id.as_str())
	.bind(now)
	.fetch_one(&mut **tx)
	.await
	.map_err(Into::into)
}

pub(super) async fn fetch_attached_block_rows<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
) -> Result<Vec<CoreBlockJoinedRow>>
where
	E: PgExecutor<'e>,
{
	sqlx::query_as::<_, CoreBlockJoinedRow>(
		"\
SELECT
	a.attachment_id,
	a.agent_id AS attachment_agent_id,
	a.attached_by_agent_id,
	a.attached_at,
	b.block_id,
	b.tenant_id,
	b.project_id,
	b.agent_id,
	b.scope,
	b.key,
	b.title,
	b.content,
	b.source_ref,
	b.status,
	b.created_at,
	b.updated_at
FROM core_memory_block_attachments a
JOIN core_memory_blocks b ON b.block_id = a.block_id
WHERE a.tenant_id = $1
	AND a.project_id = $2
	AND a.agent_id = $3
	AND a.read_profile = $4
	AND a.detached_at IS NULL
	AND b.status = 'active'
ORDER BY a.attached_at ASC, b.key ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(read_profile)
	.fetch_all(executor)
	.await
	.map_err(Into::into)
}

pub(super) async fn fetch_audit_history<'e, E>(
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

pub(super) async fn insert_core_block_event(
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
