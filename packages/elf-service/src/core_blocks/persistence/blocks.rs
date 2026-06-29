use serde_json::Value;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result,
	core_blocks::{rows::CoreBlockRow, types::PreparedUpsertRequest, validation},
};

pub(in crate::core_blocks) async fn insert_core_block(
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

pub(in crate::core_blocks) async fn update_core_block(
	tx: &mut Transaction<'_, Postgres>,
	req: &PreparedUpsertRequest,
	block_id: Uuid,
	now: OffsetDateTime,
) -> Result<(CoreBlockRow, Option<Value>)> {
	let prev = fetch_owned_block_for_update(tx, req, block_id).await?;
	let prev_snapshot = Some(validation::block_snapshot(&prev));

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
