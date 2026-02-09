CREATE TABLE IF NOT EXISTS search_trace_candidates (
	candidate_id uuid PRIMARY KEY,
	trace_id uuid NOT NULL REFERENCES search_traces(trace_id) ON DELETE CASCADE,
	note_id uuid NOT NULL,
	chunk_id uuid NOT NULL,
	retrieval_rank int NOT NULL,
	rerank_score real NOT NULL,
	note_scope text NOT NULL,
	note_importance real NOT NULL,
	note_updated_at timestamptz NOT NULL,
	created_at timestamptz NOT NULL,
	expires_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_trace_candidates_expires
	ON search_trace_candidates (expires_at);
CREATE INDEX IF NOT EXISTS idx_search_trace_candidates_trace
	ON search_trace_candidates (trace_id, retrieval_rank);

