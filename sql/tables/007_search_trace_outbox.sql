CREATE TABLE IF NOT EXISTS search_trace_outbox (
	outbox_id uuid PRIMARY KEY,
	trace_id uuid NOT NULL,
	status text NOT NULL,
	attempts int NOT NULL DEFAULT 0,
	last_error text NULL,
	available_at timestamptz NOT NULL DEFAULT now(),
	payload jsonb NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_trace_outbox_status_available
	ON search_trace_outbox (status, available_at);
CREATE INDEX IF NOT EXISTS idx_trace_outbox_trace_status
	ON search_trace_outbox (trace_id, status);
