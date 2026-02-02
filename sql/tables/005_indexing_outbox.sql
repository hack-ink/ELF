CREATE TABLE IF NOT EXISTS indexing_outbox (
    outbox_id uuid PRIMARY KEY,
    note_id uuid NOT NULL,
    op text NOT NULL,
    embedding_version text NOT NULL,
    status text NOT NULL,
    attempts int NOT NULL DEFAULT 0,
    last_error text NULL,
    available_at timestamptz NOT NULL DEFAULT now(),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_outbox_status_available
    ON indexing_outbox (status, available_at);
CREATE INDEX IF NOT EXISTS idx_outbox_note_op_status
    ON indexing_outbox (note_id, op, status);
