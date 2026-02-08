CREATE TABLE IF NOT EXISTS memory_hits (
    hit_id uuid PRIMARY KEY,
    note_id uuid NOT NULL,
    query_hash text NOT NULL,
    rank int NOT NULL,
    final_score real NOT NULL,
    ts timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE memory_hits
    ADD COLUMN IF NOT EXISTS chunk_id uuid NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_memory_hits_note_id'
    ) THEN
        ALTER TABLE memory_hits
            ADD CONSTRAINT fk_memory_hits_note_id
                FOREIGN KEY (note_id)
                REFERENCES memory_notes(note_id)
                ON DELETE CASCADE;
    END IF;
END $$;
