use sqlx::PgExecutor;

use crate::{Result, core_blocks::rows::CoreBlockJoinedRow};

pub(in crate::core_blocks) async fn fetch_attached_block_rows<'e, E>(
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
