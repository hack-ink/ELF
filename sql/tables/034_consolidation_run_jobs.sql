CREATE TABLE IF NOT EXISTS consolidation_run_jobs (
	job_id uuid PRIMARY KEY,
	run_id uuid NOT NULL REFERENCES consolidation_runs(run_id) ON DELETE CASCADE,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	job_kind text NOT NULL,
	status text NOT NULL,
	payload jsonb NOT NULL,
	attempts int NOT NULL DEFAULT 0,
	last_error text NULL,
	available_at timestamptz NOT NULL DEFAULT now(),
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE consolidation_run_jobs
	DROP CONSTRAINT IF EXISTS ck_consolidation_run_jobs_status;
ALTER TABLE consolidation_run_jobs
	ADD CONSTRAINT ck_consolidation_run_jobs_status
		CHECK (status IN ('PENDING', 'CLAIMED', 'DONE', 'FAILED'));

ALTER TABLE consolidation_run_jobs
	DROP CONSTRAINT IF EXISTS ck_consolidation_run_jobs_payload;
ALTER TABLE consolidation_run_jobs
	ADD CONSTRAINT ck_consolidation_run_jobs_payload
		CHECK (jsonb_typeof(payload) = 'object');

CREATE INDEX IF NOT EXISTS idx_consolidation_run_jobs_status_available
	ON consolidation_run_jobs (status, available_at);

CREATE INDEX IF NOT EXISTS idx_consolidation_run_jobs_run_status
	ON consolidation_run_jobs (run_id, status);
