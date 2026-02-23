CREATE TABLE IF NOT EXISTS note_field_embeddings (
	field_id uuid NOT NULL REFERENCES memory_note_fields(field_id) ON DELETE CASCADE,
	embedding_version text NOT NULL,
	embedding_dim int NOT NULL,
	vec vector(<VECTOR_DIM>) NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	PRIMARY KEY (field_id, embedding_version)
);
