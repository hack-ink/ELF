use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
	Result, consolidation::types::ConsolidationProposalReviewEventInsert,
	models::ConsolidationProposalReviewEvent,
};

/// Inserts one proposal review audit event.
pub async fn insert_consolidation_proposal_review_event<'e, E>(
	executor: E,
	args: ConsolidationProposalReviewEventInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO consolidation_proposal_reviews (
	review_id,
	proposal_id,
	run_id,
	tenant_id,
	project_id,
	reviewer_agent_id,
	action,
	from_review_state,
	to_review_state,
	review_comment,
	created_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
	)
	.bind(args.review_id)
	.bind(args.proposal_id)
	.bind(args.run_id)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.reviewer_agent_id)
	.bind(args.action)
	.bind(args.from_review_state)
	.bind(args.to_review_state)
	.bind(args.review_comment)
	.bind(args.created_at)
	.execute(executor)
	.await?;

	Ok(())
}

/// Lists review events for one consolidation proposal.
pub async fn list_consolidation_proposal_review_events<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	proposal_id: Uuid,
) -> Result<Vec<ConsolidationProposalReviewEvent>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, ConsolidationProposalReviewEvent>(
		"\
SELECT
	review_id,
	proposal_id,
	run_id,
	tenant_id,
	project_id,
	reviewer_agent_id,
	action,
	from_review_state,
	to_review_state,
	review_comment,
	created_at
FROM consolidation_proposal_reviews
WHERE tenant_id = $1 AND project_id = $2 AND proposal_id = $3
ORDER BY created_at ASC, review_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(proposal_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
