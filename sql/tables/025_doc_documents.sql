CREATE TABLE IF NOT EXISTS doc_documents (
	doc_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	scope text NOT NULL,
	status text NOT NULL,
	title text NULL,
	source_ref jsonb NULL,
	content text NOT NULL,
	content_bytes int NOT NULL,
	content_hash text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE doc_documents
	DROP CONSTRAINT IF EXISTS ck_doc_documents_scope;
ALTER TABLE doc_documents
	ADD CONSTRAINT ck_doc_documents_scope
		CHECK (scope IN ('agent_private', 'project_shared', 'org_shared'));

ALTER TABLE doc_documents
	DROP CONSTRAINT IF EXISTS ck_doc_documents_status;
ALTER TABLE doc_documents
	ADD CONSTRAINT ck_doc_documents_status
		CHECK (status IN ('active', 'deleted'));

CREATE INDEX IF NOT EXISTS idx_doc_documents_tenant_project_scope_status_updated
	ON doc_documents (tenant_id, project_id, scope, status, updated_at DESC);

