CREATE TABLE IF NOT EXISTS work_journal_entries (
	entry_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	scope text NOT NULL,
	session_id text NOT NULL,
	family text NOT NULL,
	status text NOT NULL,
	title text NULL,
	body text NOT NULL,
	source_refs jsonb NOT NULL DEFAULT '[]'::jsonb,
	explicit_next_steps jsonb NOT NULL DEFAULT '[]'::jsonb,
	inferred_next_steps jsonb NOT NULL DEFAULT '[]'::jsonb,
	rejected_options jsonb NOT NULL DEFAULT '[]'::jsonb,
	promotion_boundary jsonb NOT NULL DEFAULT '{}'::jsonb,
	redaction_audit jsonb NOT NULL DEFAULT '{}'::jsonb,
	created_at timestamptz NOT NULL,
	updated_at timestamptz NOT NULL
);

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_scope;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_scope
		CHECK (scope IN ('agent_private', 'project_shared', 'org_shared'));

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_family;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_family
		CHECK (
			family IN (
				'session_log',
				'handoff_brief',
				'janitor_report',
				'explicit_next_step',
				'inferred_next_step',
				'rejected_option'
			)
		);

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_status;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_status
		CHECK (status IN ('active'));

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_source_refs;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_source_refs
		CHECK (jsonb_typeof(source_refs) = 'array' AND jsonb_array_length(source_refs) > 0);

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_explicit_next_steps;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_explicit_next_steps
		CHECK (jsonb_typeof(explicit_next_steps) = 'array');

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_inferred_next_steps;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_inferred_next_steps
		CHECK (jsonb_typeof(inferred_next_steps) = 'array');

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_rejected_options;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_rejected_options
		CHECK (jsonb_typeof(rejected_options) = 'array');

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_promotion_boundary;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_promotion_boundary
		CHECK (jsonb_typeof(promotion_boundary) = 'object');

ALTER TABLE work_journal_entries
	DROP CONSTRAINT IF EXISTS ck_work_journal_entries_redaction_audit;
ALTER TABLE work_journal_entries
	ADD CONSTRAINT ck_work_journal_entries_redaction_audit
		CHECK (jsonb_typeof(redaction_audit) = 'object');

CREATE INDEX IF NOT EXISTS idx_work_journal_entries_session
	ON work_journal_entries (tenant_id, project_id, session_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_work_journal_entries_scope_status
	ON work_journal_entries (tenant_id, project_id, scope, status);
