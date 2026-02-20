CREATE TABLE IF NOT EXISTS memory_ingest_decisions (
	decision_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	scope text NOT NULL,
	pipeline text NOT NULL,
	note_type text NOT NULL,
	note_key text NULL,
	note_id uuid NULL,
	base_decision text NOT NULL,
	policy_decision text NOT NULL,
	note_op text NOT NULL,
	reason_code text NULL,
	details jsonb NOT NULL DEFAULT '{}'::jsonb,
	ts timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT ck_memory_ingest_decisions_pipeline
		CHECK (pipeline IN ('add_note', 'add_event')),
	CONSTRAINT ck_memory_ingest_decisions_base_decision
		CHECK (base_decision IN ('remember', 'update', 'ignore', 'reject')),
	CONSTRAINT ck_memory_ingest_decisions_policy_decision
		CHECK (policy_decision IN ('remember', 'update', 'ignore', 'reject')),
	CONSTRAINT ck_memory_ingest_decisions_note_op
		CHECK (note_op IN ('ADD', 'UPDATE', 'NONE', 'DELETE', 'REJECTED'))
);

CREATE INDEX IF NOT EXISTS idx_memory_ingest_decisions_context
	ON memory_ingest_decisions (tenant_id, project_id, agent_id, ts desc);
CREATE INDEX IF NOT EXISTS idx_memory_ingest_decisions_note_id
	ON memory_ingest_decisions (note_id);
CREATE INDEX IF NOT EXISTS idx_memory_ingest_decisions_policy_decision
	ON memory_ingest_decisions (policy_decision);
CREATE INDEX IF NOT EXISTS idx_memory_ingest_decisions_pipeline
	ON memory_ingest_decisions (pipeline);
