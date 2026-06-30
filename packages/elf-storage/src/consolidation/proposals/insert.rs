use sqlx::PgExecutor;

use crate::{Result, models::ConsolidationProposal};

/// Inserts one consolidation proposal.
pub async fn insert_consolidation_proposal<'e, E>(
	executor: E,
	proposal: &ConsolidationProposal,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO consolidation_proposals (
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
	unsupported_claim_flags,
	contradiction_markers,
	staleness_markers,
	target_ref,
	proposed_payload,
	reviewer_agent_id,
	review_comment,
	reviewed_at,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
	)
	.bind(proposal.proposal_id)
	.bind(proposal.run_id)
	.bind(proposal.tenant_id.as_str())
	.bind(proposal.project_id.as_str())
	.bind(proposal.agent_id.as_str())
	.bind(proposal.contract_schema.as_str())
	.bind(proposal.proposal_kind.as_str())
	.bind(proposal.apply_intent.as_str())
	.bind(proposal.review_state.as_str())
	.bind(&proposal.source_refs)
	.bind(&proposal.source_snapshot)
	.bind(&proposal.lineage)
	.bind(&proposal.diff)
	.bind(proposal.confidence)
	.bind(&proposal.unsupported_claim_flags)
	.bind(&proposal.contradiction_markers)
	.bind(&proposal.staleness_markers)
	.bind(&proposal.target_ref)
	.bind(&proposal.proposed_payload)
	.bind(proposal.reviewer_agent_id.as_deref())
	.bind(proposal.review_comment.as_deref())
	.bind(proposal.reviewed_at)
	.bind(proposal.created_at)
	.bind(proposal.updated_at)
	.execute(executor)
	.await?;

	Ok(())
}
