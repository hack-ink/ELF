use sqlx::PgExecutor;

use crate::{
	Result,
	consolidation::types::{
		ConsolidationProposalReviewUpdate, ConsolidationProposalTargetRefUpdate,
	},
	models::ConsolidationProposal,
};

/// Updates one proposal review state.
pub async fn update_consolidation_proposal_review<'e, E>(
	executor: E,
	args: ConsolidationProposalReviewUpdate<'_>,
) -> Result<Option<ConsolidationProposal>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationProposal>(
		"\
UPDATE consolidation_proposals
SET
	review_state = $1,
	reviewer_agent_id = $2,
	review_comment = $3,
	reviewed_at = $4,
	updated_at = $4
WHERE tenant_id = $5 AND project_id = $6 AND proposal_id = $7
RETURNING
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
	updated_at",
	)
	.bind(args.review_state)
	.bind(args.reviewer_agent_id)
	.bind(args.review_comment)
	.bind(args.now)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.proposal_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

/// Updates one proposal target reference.
pub async fn update_consolidation_proposal_target_ref<'e, E>(
	executor: E,
	args: ConsolidationProposalTargetRefUpdate<'_>,
) -> Result<Option<ConsolidationProposal>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationProposal>(
		"\
UPDATE consolidation_proposals
SET target_ref = $1, updated_at = $2
WHERE tenant_id = $3 AND project_id = $4 AND proposal_id = $5
RETURNING
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
	updated_at",
	)
	.bind(args.target_ref)
	.bind(args.now)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.proposal_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}
