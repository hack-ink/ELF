CREATE TABLE IF NOT EXISTS knowledge_page_source_refs (
	ref_id uuid PRIMARY KEY,
	page_id uuid NOT NULL REFERENCES knowledge_pages(page_id) ON DELETE CASCADE,
	section_id uuid NULL REFERENCES knowledge_page_sections(section_id) ON DELETE CASCADE,
	source_kind text NOT NULL,
	source_id uuid NOT NULL,
	source_status text NULL,
	source_updated_at timestamptz NULL,
	source_content_hash text NULL,
	source_snapshot jsonb NOT NULL DEFAULT '{}'::jsonb,
	citation_metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
	created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE knowledge_page_source_refs
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_source_refs_source_kind;
ALTER TABLE knowledge_page_source_refs
	ADD CONSTRAINT ck_knowledge_page_source_refs_source_kind
		CHECK (source_kind IN ('note', 'event', 'relation', 'proposal'));

ALTER TABLE knowledge_page_source_refs
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_source_refs_source_snapshot;
ALTER TABLE knowledge_page_source_refs
	ADD CONSTRAINT ck_knowledge_page_source_refs_source_snapshot
		CHECK (jsonb_typeof(source_snapshot) = 'object');

ALTER TABLE knowledge_page_source_refs
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_source_refs_citation_metadata;
ALTER TABLE knowledge_page_source_refs
	ADD CONSTRAINT ck_knowledge_page_source_refs_citation_metadata
		CHECK (jsonb_typeof(citation_metadata) = 'object');

CREATE INDEX IF NOT EXISTS idx_knowledge_page_source_refs_page
	ON knowledge_page_source_refs (page_id, source_kind, source_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_page_source_refs_source
	ON knowledge_page_source_refs (source_kind, source_id);
