CREATE TABLE IF NOT EXISTS memory_note_fields (
	field_id uuid PRIMARY KEY,
	note_id uuid NOT NULL REFERENCES memory_notes(note_id) ON DELETE CASCADE,
	field_kind text NOT NULL,
	item_index int NOT NULL,
	text text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_note_fields_note_kind_index
	ON memory_note_fields (note_id, field_kind, item_index);
CREATE INDEX IF NOT EXISTS idx_note_fields_note
	ON memory_note_fields (note_id);
CREATE INDEX IF NOT EXISTS idx_note_fields_kind
	ON memory_note_fields (field_kind);

