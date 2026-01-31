CREATE EXTENSION IF NOT EXISTS vector;

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

CREATE TABLE IF NOT EXISTS note_embeddings (
    note_id uuid NOT NULL REFERENCES memory_notes(note_id) ON DELETE CASCADE,
    embedding_version text NOT NULL,
    embedding_dim int NOT NULL,
    vec vector(<VECTOR_DIM>) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (note_id, embedding_version)
);

CREATE TABLE IF NOT EXISTS memory_note_versions (
    version_id uuid PRIMARY KEY,
    note_id uuid NOT NULL,
    op text NOT NULL,
    prev_snapshot jsonb NULL,
    new_snapshot jsonb NULL,
    reason text NOT NULL,
    actor text NOT NULL,
    ts timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS memory_hits (
    hit_id uuid PRIMARY KEY,
    note_id uuid NOT NULL,
    query_hash text NOT NULL,
    rank int NOT NULL,
    final_score real NOT NULL,
    ts timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS indexing_outbox (
    outbox_id uuid PRIMARY KEY,
    note_id uuid NOT NULL,
    op text NOT NULL,
    embedding_version text NOT NULL,
    status text NOT NULL,
    attempts int NOT NULL DEFAULT 0,
    last_error text NULL,
    available_at timestamptz NOT NULL DEFAULT now(),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_notes_scope_status
    ON memory_notes (tenant_id, project_id, scope, status);
CREATE INDEX IF NOT EXISTS idx_notes_key
    ON memory_notes (tenant_id, project_id, agent_id, scope, type, key)
    WHERE key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_notes_expires
    ON memory_notes (expires_at);

CREATE INDEX IF NOT EXISTS idx_outbox_status_available
    ON indexing_outbox (status, available_at);
CREATE INDEX IF NOT EXISTS idx_outbox_note_op_status
    ON indexing_outbox (note_id, op, status);
