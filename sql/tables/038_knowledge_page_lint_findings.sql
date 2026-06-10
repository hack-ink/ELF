CREATE TABLE IF NOT EXISTS knowledge_page_lint_findings (
	finding_id uuid PRIMARY KEY,
	page_id uuid NOT NULL REFERENCES knowledge_pages(page_id) ON DELETE CASCADE,
	section_id uuid NULL REFERENCES knowledge_page_sections(section_id) ON DELETE SET NULL,
	finding_type text NOT NULL,
	severity text NOT NULL,
	source_kind text NULL,
	source_id uuid NULL,
	message text NOT NULL,
	details jsonb NOT NULL DEFAULT '{}'::jsonb,
	created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE knowledge_page_lint_findings
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_lint_findings_severity;
ALTER TABLE knowledge_page_lint_findings
	ADD CONSTRAINT ck_knowledge_page_lint_findings_severity
		CHECK (severity IN ('info', 'warning', 'error'));

ALTER TABLE knowledge_page_lint_findings
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_lint_findings_source_kind;
ALTER TABLE knowledge_page_lint_findings
	ADD CONSTRAINT ck_knowledge_page_lint_findings_source_kind
		CHECK (source_kind IS NULL OR source_kind IN ('note', 'event', 'relation', 'proposal'));

ALTER TABLE knowledge_page_lint_findings
	DROP CONSTRAINT IF EXISTS ck_knowledge_page_lint_findings_details;
ALTER TABLE knowledge_page_lint_findings
	ADD CONSTRAINT ck_knowledge_page_lint_findings_details
		CHECK (jsonb_typeof(details) = 'object');

CREATE INDEX IF NOT EXISTS idx_knowledge_page_lint_findings_page
	ON knowledge_page_lint_findings (page_id, severity, created_at DESC);
