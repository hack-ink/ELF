CREATE TABLE IF NOT EXISTS note_chunk_embeddings (
	chunk_id uuid NOT NULL REFERENCES memory_note_chunks(chunk_id) ON DELETE CASCADE,
	embedding_version text NOT NULL,
	embedding_dim int NOT NULL,
	vec vector(<VECTOR_DIM>) NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	PRIMARY KEY (chunk_id, embedding_version)
);
