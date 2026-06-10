CREATE TABLE IF NOT EXISTS knowledge_pages (
	page_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	page_kind text NOT NULL,
	page_key text NOT NULL,
	title text NOT NULL,
	contract_schema text NOT NULL,
	status text NOT NULL,
	rebuild_source_hash text NOT NULL,
	content_hash text NOT NULL,
	source_coverage jsonb NOT NULL DEFAULT '{}'::jsonb,
	source_snapshot jsonb NOT NULL DEFAULT '{}'::jsonb,
	rebuild_metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now(),
	rebuilt_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE knowledge_pages
	DROP CONSTRAINT IF EXISTS ck_knowledge_pages_page_kind;
ALTER TABLE knowledge_pages
	ADD CONSTRAINT ck_knowledge_pages_page_kind
		CHECK (page_kind IN ('project', 'entity', 'concept', 'issue', 'decision'));

ALTER TABLE knowledge_pages
	DROP CONSTRAINT IF EXISTS ck_knowledge_pages_status;
ALTER TABLE knowledge_pages
	ADD CONSTRAINT ck_knowledge_pages_status
		CHECK (status IN ('active', 'stale', 'archived'));

ALTER TABLE knowledge_pages
	DROP CONSTRAINT IF EXISTS ck_knowledge_pages_source_coverage;
ALTER TABLE knowledge_pages
	ADD CONSTRAINT ck_knowledge_pages_source_coverage
		CHECK (jsonb_typeof(source_coverage) = 'object');

ALTER TABLE knowledge_pages
	DROP CONSTRAINT IF EXISTS ck_knowledge_pages_source_snapshot;
ALTER TABLE knowledge_pages
	ADD CONSTRAINT ck_knowledge_pages_source_snapshot
		CHECK (jsonb_typeof(source_snapshot) = 'object');

ALTER TABLE knowledge_pages
	DROP CONSTRAINT IF EXISTS ck_knowledge_pages_rebuild_metadata;
ALTER TABLE knowledge_pages
	ADD CONSTRAINT ck_knowledge_pages_rebuild_metadata
		CHECK (jsonb_typeof(rebuild_metadata) = 'object');

CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_pages_context_key
	ON knowledge_pages (tenant_id, project_id, page_kind, page_key);

CREATE INDEX IF NOT EXISTS idx_knowledge_pages_context_updated
	ON knowledge_pages (tenant_id, project_id, updated_at DESC);
