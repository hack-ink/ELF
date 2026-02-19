CREATE TABLE IF NOT EXISTS graph_fact_supersessions (
	supersession_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	from_fact_id uuid NOT NULL REFERENCES graph_facts(fact_id) ON DELETE CASCADE,
	to_fact_id uuid NOT NULL REFERENCES graph_facts(fact_id) ON DELETE CASCADE,
	note_id uuid NOT NULL REFERENCES memory_notes(note_id) ON DELETE CASCADE,
	effective_at timestamptz NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_graph_fact_supersessions_from_to_note
	ON graph_fact_supersessions (from_fact_id, to_fact_id, note_id);

CREATE INDEX IF NOT EXISTS idx_graph_fact_supersessions_from_fact
	ON graph_fact_supersessions (from_fact_id);
CREATE INDEX IF NOT EXISTS idx_graph_fact_supersessions_to_fact
	ON graph_fact_supersessions (to_fact_id);
CREATE INDEX IF NOT EXISTS idx_graph_fact_supersessions_note
	ON graph_fact_supersessions (note_id);

