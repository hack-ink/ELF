CREATE TABLE IF NOT EXISTS consolidation_proposal_reviews (
	review_id uuid PRIMARY KEY,
	proposal_id uuid NOT NULL REFERENCES consolidation_proposals(proposal_id) ON DELETE CASCADE,
	run_id uuid NOT NULL REFERENCES consolidation_runs(run_id) ON DELETE CASCADE,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	reviewer_agent_id text NOT NULL,
	action text NOT NULL,
	from_review_state text NOT NULL,
	to_review_state text NOT NULL,
	review_comment text NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE consolidation_proposal_reviews
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposal_reviews_action;
ALTER TABLE consolidation_proposal_reviews
	ADD CONSTRAINT ck_consolidation_proposal_reviews_action
		CHECK (action IN ('approve', 'apply', 'discard', 'defer'));

ALTER TABLE consolidation_proposal_reviews
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposal_reviews_from_state;
ALTER TABLE consolidation_proposal_reviews
	ADD CONSTRAINT ck_consolidation_proposal_reviews_from_state
		CHECK (from_review_state IN ('proposed', 'approved', 'rejected', 'applied', 'archived'));

ALTER TABLE consolidation_proposal_reviews
	DROP CONSTRAINT IF EXISTS ck_consolidation_proposal_reviews_to_state;
ALTER TABLE consolidation_proposal_reviews
	ADD CONSTRAINT ck_consolidation_proposal_reviews_to_state
		CHECK (to_review_state IN ('proposed', 'approved', 'rejected', 'applied', 'archived'));

CREATE INDEX IF NOT EXISTS idx_consolidation_proposal_reviews_proposal_created
	ON consolidation_proposal_reviews (proposal_id, created_at ASC, review_id ASC);

CREATE INDEX IF NOT EXISTS idx_consolidation_proposal_reviews_context_created
	ON consolidation_proposal_reviews (tenant_id, project_id, created_at DESC);
