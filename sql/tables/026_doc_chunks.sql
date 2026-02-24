CREATE TABLE IF NOT EXISTS doc_chunks (
	chunk_id uuid PRIMARY KEY,
	doc_id uuid NOT NULL REFERENCES doc_documents(doc_id) ON DELETE CASCADE,
	chunk_index int NOT NULL,
	start_offset int NOT NULL,
	end_offset int NOT NULL,
	chunk_text text NOT NULL,
	chunk_hash text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE doc_chunks
	DROP CONSTRAINT IF EXISTS ck_doc_chunks_offsets;
ALTER TABLE doc_chunks
	ADD CONSTRAINT ck_doc_chunks_offsets
		CHECK (start_offset >= 0 AND end_offset >= start_offset);

CREATE UNIQUE INDEX IF NOT EXISTS uq_doc_chunks_doc_index
	ON doc_chunks (doc_id, chunk_index);

CREATE INDEX IF NOT EXISTS idx_doc_chunks_doc_id
	ON doc_chunks (doc_id);

