CREATE TABLE IF NOT EXISTS memory_notes (
    note_id uuid PRIMARY KEY,
    tenant_id text NOT NULL,
    project_id text NOT NULL,
    agent_id text NOT NULL,
    scope text NOT NULL,
    type text NOT NULL,
    key text NULL,
    text text NOT NULL,
    importance real NOT NULL,
    confidence real NOT NULL,
    status text NOT NULL,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    expires_at timestamptz NULL,
    embedding_version text NOT NULL,
    source_ref jsonb NOT NULL,
    hit_count bigint NOT NULL DEFAULT 0,
    last_hit_at timestamptz NULL
);

CREATE INDEX IF NOT EXISTS idx_notes_scope_status
    ON memory_notes (tenant_id, project_id, scope, status);
CREATE INDEX IF NOT EXISTS idx_notes_key
    ON memory_notes (tenant_id, project_id, agent_id, scope, type, key)
    WHERE key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_notes_expires
    ON memory_notes (expires_at);
