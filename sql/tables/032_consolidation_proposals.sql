CREATE TABLE IF NOT EXISTS consolidation_proposals (
	proposal_id uuid PRIMARY KEY,
	run_id uuid NOT NULL REFERENCES consolidation_runs(run_id) ON DELETE CASCADE,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	contract_schema text NOT NULL,
	proposal_kind text NOT NULL,
	apply_intent text NOT NULL,
	review_state text NOT NULL,
	source_refs jsonb NOT NULL,
	source_snapshot jsonb NOT NULL,
	lineage jsonb NOT NULL,
	diff jsonb NOT NULL,
	confidence real NOT NULL,
	contradiction_markers jsonb NOT NULL DEFAULT '[]'::jsonb,
	staleness_markers jsonb NOT NULL DEFAULT '[]'::jsonb,
	target_ref jsonb NOT NULL DEFAULT '{}'::jsonb,
	proposed_payload jsonb NOT NULL DEFAULT '{}'::jsonb,
	reviewer_agent_id text NULL,
	review_comment text NULL,
	reviewed_at timestamptz NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_apply_intent;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_apply_intent
		CHECK (
			apply_intent IN (
				'create_derived_note',
				'update_derived_note',
				'create_derived_knowledge_page',
				'update_derived_knowledge_page',
				'create_derived_graph_view',
				'no_op'
			)
		);

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_review_state;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_review_state
		CHECK (review_state IN ('proposed', 'approved', 'rejected', 'applied', 'archived'));

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_source_refs;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_source_refs
		CHECK (jsonb_typeof(source_refs) = 'array');

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_source_snapshot;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_source_snapshot
		CHECK (jsonb_typeof(source_snapshot) = 'object');

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_lineage;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_lineage
		CHECK (jsonb_typeof(lineage) = 'object');

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_diff;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_diff
		CHECK (jsonb_typeof(diff) = 'object');

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_confidence;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_confidence
		CHECK (confidence >= 0.0 AND confidence <= 1.0);

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_contradiction_markers;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_contradiction_markers
		CHECK (jsonb_typeof(contradiction_markers) = 'array');

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_staleness_markers;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_staleness_markers
		CHECK (jsonb_typeof(staleness_markers) = 'array');

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_target_ref;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_target_ref
		CHECK (jsonb_typeof(target_ref) = 'object');

ALTER TABLE consolidation_proposals
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposals_proposed_payload;
ALTER TABLE consolidation_proposals
	ADD CONSTRAINT ck_consolidation_proposals_proposed_payload
		CHECK (jsonb_typeof(proposed_payload) = 'object');

CREATE INDEX IF NOT EXISTS idx_consolidation_proposals_run_created
	ON consolidation_proposals (run_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_consolidation_proposals_context_state_created
	ON consolidation_proposals (tenant_id, project_id, review_state, created_at DESC);
