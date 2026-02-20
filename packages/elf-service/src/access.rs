use std::collections::HashSet;

use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use elf_storage::models::MemoryNote;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct SharedSpaceGrantKey {
	pub(crate) scope: String,
	pub(crate) space_owner_agent_id: String,
}

pub(crate) async fn load_shared_read_grants<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	grantee_agent_id: &str,
) -> Result<HashSet<SharedSpaceGrantKey>>
where
	E: PgExecutor<'e>,
{
	let rows: Vec<(String, String)> = sqlx::query_as(
		"\
SELECT scope, space_owner_agent_id
FROM memory_space_grants
WHERE tenant_id = $1
  AND project_id = $2
  AND revoked_at IS NULL
  AND scope IN ('project_shared', 'org_shared')
  AND (
    grantee_kind = 'project'
    OR (grantee_kind = 'agent' AND grantee_agent_id = $3)
  )",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(grantee_agent_id)
	.fetch_all(executor)
	.await?;
	let mut grants = HashSet::with_capacity(rows.len());

	for (scope, space_owner_agent_id) in rows {
		grants.insert(SharedSpaceGrantKey { scope, space_owner_agent_id });
	}

	Ok(grants)
}

pub(crate) fn note_read_allowed(
	note: &MemoryNote,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	now: OffsetDateTime,
) -> bool {
	if note.status != "active" {
		return false;
	}
	if note.expires_at.map(|expires_at| expires_at <= now).unwrap_or(false) {
		return false;
	}
	if !allowed_scopes.iter().any(|scope| scope == &note.scope) {
		return false;
	}
	if note.scope == "agent_private" {
		return note.agent_id == requester_agent_id;
	}

	if !is_shared_scope(note.scope.as_str()) {
		return false;
	}
	if note.agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&SharedSpaceGrantKey {
		scope: note.scope.clone(),
		space_owner_agent_id: note.agent_id.clone(),
	})
}

pub(crate) async fn ensure_active_project_scope_grant<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	scope: &str,
	space_owner_agent_id: &str,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if !is_shared_scope(scope) {
		return Ok(());
	}

	sqlx::query(
		"\
INSERT INTO memory_space_grants (
\tgrant_id,
\ttenant_id,
\tproject_id,
\tscope,
\tspace_owner_agent_id,
\tgrantee_kind,
\tgrantee_agent_id,
\tgranted_by_agent_id,
\tgranted_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (tenant_id, project_id, scope, space_owner_agent_id)
WHERE revoked_at IS NULL AND grantee_kind='project'
DO UPDATE
SET
\tgranted_by_agent_id = EXCLUDED.granted_by_agent_id,
\tgranted_at = EXCLUDED.granted_at,
\trevoked_at = NULL,
\trevoked_by_agent_id = NULL",
	)
	.bind(Uuid::new_v4())
	.bind(tenant_id)
	.bind(project_id)
	.bind(scope)
	.bind(space_owner_agent_id)
	.bind("project")
	.bind::<Option<&str>>(None)
	.bind(space_owner_agent_id)
	.bind(OffsetDateTime::now_utc())
	.execute(executor)
	.await?;

	Ok(())
}

fn is_shared_scope(scope: &str) -> bool {
	matches!(scope, "project_shared" | "org_shared")
}
