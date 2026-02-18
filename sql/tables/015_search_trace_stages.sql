CREATE TABLE IF NOT EXISTS search_trace_stages (
	stage_id uuid PRIMARY KEY,
	trace_id uuid NOT NULL REFERENCES search_traces(trace_id) ON DELETE CASCADE,
	stage_order int NOT NULL,
	stage_name text NOT NULL,
	stage_payload jsonb NOT NULL,
	created_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_trace_stages_trace_order
	ON search_trace_stages (trace_id, stage_order);
CREATE INDEX IF NOT EXISTS idx_search_trace_stages_trace_name
	ON search_trace_stages (trace_id, stage_name);

CREATE TABLE IF NOT EXISTS search_trace_stage_items (
	id uuid PRIMARY KEY,
	stage_id uuid NOT NULL REFERENCES search_trace_stages(stage_id) ON DELETE CASCADE,
	item_id uuid NULL,
	note_id uuid NULL,
	chunk_id uuid NULL,
	metrics jsonb NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_trace_stage_items_stage_item
	ON search_trace_stage_items (stage_id, item_id);
