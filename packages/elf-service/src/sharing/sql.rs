pub(super) const PROJECT_SPACE_GRANT_UPSERT_SQL: &str = "\
INSERT INTO memory_space_grants (
	grant_id,
	tenant_id,
	project_id,
	scope,
	space_owner_agent_id,
	grantee_kind,
	grantee_agent_id,
	granted_by_agent_id,
	granted_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9
)
ON CONFLICT (tenant_id, project_id, scope, space_owner_agent_id)
WHERE revoked_at IS NULL AND grantee_kind = 'project'
DO UPDATE
SET
	granted_by_agent_id = EXCLUDED.granted_by_agent_id,
	granted_at = EXCLUDED.granted_at,
	revoked_at = NULL,
	revoked_by_agent_id = NULL";

pub(super) const AGENT_SPACE_GRANT_UPSERT_SQL: &str = "\
INSERT INTO memory_space_grants (
	grant_id,
	tenant_id,
	project_id,
	scope,
	space_owner_agent_id,
	grantee_kind,
	grantee_agent_id,
	granted_by_agent_id,
	granted_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9
)
ON CONFLICT (tenant_id, project_id, scope, space_owner_agent_id, grantee_agent_id)
WHERE revoked_at IS NULL AND grantee_kind = 'agent'
DO UPDATE
SET
	granted_by_agent_id = EXCLUDED.granted_by_agent_id,
	granted_at = EXCLUDED.granted_at,
	revoked_at = NULL,
	revoked_by_agent_id = NULL";
