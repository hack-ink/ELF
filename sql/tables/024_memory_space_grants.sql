CREATE TABLE IF NOT EXISTS memory_space_grants (
	grant_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	scope text NOT NULL,
	space_owner_agent_id text NOT NULL,
	grantee_kind text NOT NULL,
	grantee_agent_id text NULL,
	granted_by_agent_id text NOT NULL,
	granted_at timestamptz NOT NULL DEFAULT now(),
	revoked_by_agent_id text NULL,
	revoked_at timestamptz NULL,
	CONSTRAINT ck_memory_space_grants_scope
		CHECK (scope IN ('project_shared', 'org_shared')),
	CONSTRAINT ck_memory_space_grants_grantee_kind
		CHECK (grantee_kind IN ('agent', 'project')),
	CONSTRAINT ck_memory_space_grants_grantee_agent_id_by_kind
		CHECK (
			(grantee_kind = 'agent' AND grantee_agent_id IS NOT NULL)
			OR (grantee_kind = 'project' AND grantee_agent_id IS NULL)
		),
	CONSTRAINT ck_memory_space_grants_owner_not_grantee_agent
		CHECK (NOT (grantee_kind = 'agent' AND space_owner_agent_id = grantee_agent_id))
);

DROP INDEX IF EXISTS uq_memory_space_grants_active_grant;

CREATE UNIQUE INDEX IF NOT EXISTS uq_memory_space_grants_active_agent_grant
	ON memory_space_grants (
		tenant_id,
		project_id,
		scope,
		space_owner_agent_id,
		grantee_agent_id
	)
	WHERE revoked_at IS NULL AND grantee_kind = 'agent';

CREATE UNIQUE INDEX IF NOT EXISTS uq_memory_space_grants_active_project_grant
	ON memory_space_grants (
		tenant_id,
		project_id,
		scope,
		space_owner_agent_id
	)
	WHERE revoked_at IS NULL AND grantee_kind = 'project';

CREATE INDEX IF NOT EXISTS idx_memory_space_grants_lookup_by_grantee
	ON memory_space_grants (tenant_id, project_id, grantee_kind, grantee_agent_id, scope);
CREATE INDEX IF NOT EXISTS idx_memory_space_grants_lookup_by_owner
	ON memory_space_grants (tenant_id, project_id, scope, space_owner_agent_id);
