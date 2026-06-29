pub(super) const CONSOLIDATION_RUN_SELECT: &str = "\
SELECT
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	job_kind,
	status,
	input_refs,
	source_snapshot,
	lineage,
	COALESCE(error, '{}'::jsonb) AS error,
	created_at,
	updated_at,
	completed_at
FROM consolidation_runs
WHERE tenant_id = $1 AND project_id = $2 AND run_id = $3
LIMIT 1";
pub(super) const CONSOLIDATION_PROPOSAL_SELECT: &str = "\
SELECT
	proposal_id,
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	proposal_kind,
	apply_intent,
	review_state,
	source_refs,
	source_snapshot,
	lineage,
	diff,
	confidence,
	COALESCE(unsupported_claim_flags, '[]'::jsonb) AS unsupported_claim_flags,
	COALESCE(contradiction_markers, '[]'::jsonb) AS contradiction_markers,
	COALESCE(staleness_markers, '[]'::jsonb) AS staleness_markers,
	COALESCE(target_ref, '{}'::jsonb) AS target_ref,
	COALESCE(proposed_payload, '{}'::jsonb) AS proposed_payload,
	reviewer_agent_id,
	review_comment,
	reviewed_at,
	created_at,
	updated_at
FROM consolidation_proposals
WHERE tenant_id = $1 AND project_id = $2 AND proposal_id = $3
LIMIT 1";
