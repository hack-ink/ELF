CREATE TABLE IF NOT EXISTS consolidation_runs (
	run_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	contract_schema text NOT NULL,
	job_kind text NOT NULL,
	status text NOT NULL,
	input_refs jsonb NOT NULL,
	source_snapshot jsonb NOT NULL,
	lineage jsonb NOT NULL,
	error jsonb NOT NULL DEFAULT '{}'::jsonb,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now(),
	completed_at timestamptz NULL
);

ALTER TABLE consolidation_runs
	DROP CONSTRAINT IF EXISTS ck_consolidation_runs_status;
ALTER TABLE consolidation_runs
	ADD CONSTRAINT ck_consolidation_runs_status
		CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'));

ALTER TABLE consolidation_runs
	DROP CONSTRAINT IF EXISTS ck_consolidation_runs_input_refs;
ALTER TABLE consolidation_runs
	ADD CONSTRAINT ck_consolidation_runs_input_refs
		CHECK (jsonb_typeof(input_refs) = 'array');

ALTER TABLE consolidation_runs
	DROP CONSTRAINT IF EXISTS ck_consolidation_runs_source_snapshot;
ALTER TABLE consolidation_runs
	ADD CONSTRAINT ck_consolidation_runs_source_snapshot
		CHECK (jsonb_typeof(source_snapshot) = 'object');

ALTER TABLE consolidation_runs
	DROP CONSTRAINT IF EXISTS ck_consolidation_runs_lineage;
ALTER TABLE consolidation_runs
	ADD CONSTRAINT ck_consolidation_runs_lineage
		CHECK (jsonb_typeof(lineage) = 'object');

ALTER TABLE consolidation_runs
	DROP CONSTRAINT IF EXISTS ck_consolidation_runs_error;
ALTER TABLE consolidation_runs
	ADD CONSTRAINT ck_consolidation_runs_error
		CHECK (jsonb_typeof(error) = 'object');

CREATE INDEX IF NOT EXISTS idx_consolidation_runs_context_created
	ON consolidation_runs (tenant_id, project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_consolidation_runs_status_updated
	ON consolidation_runs (tenant_id, project_id, status, updated_at DESC);
