CREATE TABLE IF NOT EXISTS core_memory_blocks (
	block_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	scope text NOT NULL,
	key text NOT NULL,
	title text NOT NULL,
	content text NOT NULL,
	source_ref jsonb NOT NULL,
	status text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT ck_core_memory_blocks_scope
		CHECK (scope IN ('agent_private', 'project_shared', 'org_shared')),
	CONSTRAINT ck_core_memory_blocks_status
		CHECK (status IN ('active', 'archived')),
	CONSTRAINT ck_core_memory_blocks_source_ref_object
		CHECK (jsonb_typeof(source_ref) = 'object')
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_core_memory_blocks_active_key
	ON core_memory_blocks (tenant_id, project_id, agent_id, scope, key)
	WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_core_memory_blocks_scope_status
	ON core_memory_blocks (tenant_id, project_id, scope, status);
