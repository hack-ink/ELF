CREATE TABLE IF NOT EXISTS llm_cache (
	cache_id uuid PRIMARY KEY,
	cache_kind text NOT NULL,
	cache_key text NOT NULL,
	payload jsonb NOT NULL,
	created_at timestamptz NOT NULL,
	last_accessed_at timestamptz NOT NULL,
	expires_at timestamptz NOT NULL,
	hit_count bigint NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_cache_key
	ON llm_cache (cache_kind, cache_key);
CREATE INDEX IF NOT EXISTS idx_llm_cache_expires
	ON llm_cache (expires_at);
