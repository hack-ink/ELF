CREATE TABLE IF NOT EXISTS search_traces (
    trace_id uuid PRIMARY KEY,
    tenant_id text NOT NULL,
    project_id text NOT NULL,
    agent_id text NOT NULL,
    read_profile text NOT NULL,
    query text NOT NULL,
    expansion_mode text NOT NULL,
    expanded_queries jsonb NOT NULL,
    allowed_scopes jsonb NOT NULL,
    candidate_count int NOT NULL,
    top_k int NOT NULL,
    config_snapshot jsonb NOT NULL,
    trace_version int NOT NULL,
    created_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_traces_expires
    ON search_traces (expires_at);
CREATE INDEX IF NOT EXISTS idx_search_traces_context
    ON search_traces (tenant_id, project_id, created_at);

CREATE TABLE IF NOT EXISTS search_trace_items (
    item_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES search_traces(trace_id) ON DELETE CASCADE,
    note_id uuid NOT NULL,
    rank int NOT NULL,
    retrieval_score real NULL,
    retrieval_rank int NULL,
    rerank_score real NOT NULL,
    tie_breaker_score real NOT NULL,
    final_score real NOT NULL,
    boosts jsonb NOT NULL,
    matched_terms jsonb NOT NULL,
    matched_fields jsonb NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_trace_items_trace
    ON search_trace_items (trace_id, rank);
CREATE INDEX IF NOT EXISTS idx_search_trace_items_note
    ON search_trace_items (note_id);
