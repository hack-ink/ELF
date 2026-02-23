CREATE TABLE IF NOT EXISTS note_embeddings (
	note_id uuid NOT NULL REFERENCES memory_notes(note_id) ON DELETE CASCADE,
	embedding_version text NOT NULL,
	embedding_dim int NOT NULL,
	vec vector(:VECTOR_DIM) NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	PRIMARY KEY (note_id, embedding_version)
);
