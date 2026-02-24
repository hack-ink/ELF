CREATE TABLE IF NOT EXISTS doc_indexing_outbox (
	outbox_id uuid PRIMARY KEY,
	doc_id uuid NOT NULL REFERENCES doc_documents(doc_id) ON DELETE CASCADE,
	chunk_id uuid NOT NULL REFERENCES doc_chunks(chunk_id) ON DELETE CASCADE,
	op text NOT NULL,
	embedding_version text NOT NULL,
	status text NOT NULL,
	attempts int NOT NULL DEFAULT 0,
	last_error text NULL,
	available_at timestamptz NOT NULL DEFAULT now(),
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE doc_indexing_outbox
	DROP CONSTRAINT IF EXISTS ck_doc_indexing_outbox_op;
ALTER TABLE doc_indexing_outbox
	ADD CONSTRAINT ck_doc_indexing_outbox_op
		CHECK (op IN ('UPSERT', 'DELETE'));

ALTER TABLE doc_indexing_outbox
	DROP CONSTRAINT IF EXISTS ck_doc_indexing_outbox_status;
ALTER TABLE doc_indexing_outbox
	ADD CONSTRAINT ck_doc_indexing_outbox_status
		CHECK (status IN ('PENDING', 'CLAIMED', 'DONE', 'FAILED'));

CREATE INDEX IF NOT EXISTS idx_doc_outbox_status_available
	ON doc_indexing_outbox (status, available_at);
CREATE INDEX IF NOT EXISTS idx_doc_outbox_doc_op_status
	ON doc_indexing_outbox (doc_id, op, status);
CREATE INDEX IF NOT EXISTS idx_doc_outbox_chunk_op_status
	ON doc_indexing_outbox (chunk_id, op, status);

