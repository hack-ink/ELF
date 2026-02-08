CREATE TABLE IF NOT EXISTS search_sessions (
	search_session_id uuid PRIMARY KEY,
	trace_id uuid NOT NULL,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	read_profile text NOT NULL,
	query text NOT NULL,
	items jsonb NOT NULL,
	created_at timestamptz NOT NULL,
	expires_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_sessions_expires
	ON search_sessions (expires_at);
CREATE INDEX IF NOT EXISTS idx_search_sessions_context
	ON search_sessions (tenant_id, project_id, created_at);

