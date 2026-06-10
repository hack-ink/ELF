CREATE TABLE IF NOT EXISTS core_memory_block_attachments (
	attachment_id uuid PRIMARY KEY,
	block_id uuid NOT NULL REFERENCES core_memory_blocks(block_id) ON DELETE CASCADE,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	read_profile text NOT NULL,
	attached_by_agent_id text NOT NULL,
	attached_at timestamptz NOT NULL DEFAULT now(),
	detached_by_agent_id text NULL,
	detached_at timestamptz NULL,
	CONSTRAINT ck_core_memory_block_attachments_read_profile
		CHECK (read_profile IN ('private_only', 'private_plus_project', 'all_scopes'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_core_memory_block_attachments_active
	ON core_memory_block_attachments (tenant_id, project_id, agent_id, read_profile, block_id)
	WHERE detached_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_core_memory_block_attachments_read
	ON core_memory_block_attachments (tenant_id, project_id, agent_id, read_profile, detached_at);

CREATE INDEX IF NOT EXISTS idx_core_memory_block_attachments_block
	ON core_memory_block_attachments (block_id, detached_at);
