CREATE TABLE IF NOT EXISTS memory_note_chunks (
	chunk_id uuid PRIMARY KEY,
	note_id uuid NOT NULL REFERENCES memory_notes(note_id) ON DELETE CASCADE,
	chunk_index int NOT NULL,
	start_offset int NOT NULL,
	end_offset int NOT NULL,
	text text NOT NULL,
	embedding_version text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_note_chunks_note
	ON memory_note_chunks (note_id);
CREATE INDEX IF NOT EXISTS idx_note_chunks_note_index
	ON memory_note_chunks (note_id, chunk_index);
