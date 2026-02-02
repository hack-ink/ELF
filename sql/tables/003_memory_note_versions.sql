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
