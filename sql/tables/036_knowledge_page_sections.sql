CREATE TABLE IF NOT EXISTS knowledge_page_sections (
	section_id uuid PRIMARY KEY,
	page_id uuid NOT NULL REFERENCES knowledge_pages(page_id) ON DELETE CASCADE,
	section_key text NOT NULL,
	heading text NOT NULL,
	role text NOT NULL,
	content text NOT NULL,
	ordinal int NOT NULL,
	citations jsonb NOT NULL DEFAULT '[]'::jsonb,
	unsupported_reason text NULL,
	content_hash text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE knowledge_page_sections
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_sections_citations;
ALTER TABLE knowledge_page_sections
	ADD CONSTRAINT ck_knowledge_page_sections_citations
		CHECK (jsonb_typeof(citations) = 'array');

ALTER TABLE knowledge_page_sections
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_sections_cited_or_unsupported;
ALTER TABLE knowledge_page_sections
	ADD CONSTRAINT ck_knowledge_page_sections_cited_or_unsupported
		CHECK (jsonb_array_length(citations) > 0 OR unsupported_reason IS NOT NULL);

CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_page_sections_page_key
	ON knowledge_page_sections (page_id, section_key);

CREATE INDEX IF NOT EXISTS idx_knowledge_page_sections_page_ordinal
	ON knowledge_page_sections (page_id, ordinal);
