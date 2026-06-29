use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result,
	access::ORG_PROJECT_ID,
	core_blocks::types::{
		CoreBlockAttachmentRow, CoreBlockRow, PreparedAttachRequest, PreparedDetachRequest,
	},
};

pub(in crate::core_blocks) async fn fetch_active_block_for_attachment(
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

pub(in crate::core_blocks) async fn upsert_core_block_attachment(
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

pub(in crate::core_blocks) async fn fetch_active_attachment_for_update(
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

pub(in crate::core_blocks) async fn detach_core_block_attachment(
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
