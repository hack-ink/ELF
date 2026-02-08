CREATE TABLE IF NOT EXISTS memory_note_versions (
    version_id uuid PRIMARY KEY,
    note_id uuid NOT NULL,
    op text NOT NULL,
    prev_snapshot jsonb NULL,
    new_snapshot jsonb NULL,
    reason text NOT NULL,
    actor text NOT NULL,
    ts timestamptz NOT NULL DEFAULT now()
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_memory_note_versions_note_id'
    ) THEN
        ALTER TABLE memory_note_versions
            ADD CONSTRAINT fk_memory_note_versions_note_id
                FOREIGN KEY (note_id)
                REFERENCES memory_notes(note_id)
                ON DELETE CASCADE;
    END IF;
END $$;
