use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
	Result, consolidation::sql::CONSOLIDATION_PROPOSAL_SELECT, models::ConsolidationProposal,
};

/// Fetches one consolidation proposal by tenant and proposal identifier.
pub async fn get_consolidation_proposal<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	proposal_id: Uuid,
) -> Result<Option<ConsolidationProposal>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationProposal>(CONSOLIDATION_PROPOSAL_SELECT)
		.bind(tenant_id)
		.bind(project_id)
		.bind(proposal_id)
		.fetch_optional(executor)
		.await?;

	Ok(row)
}

/// Locks one consolidation proposal by tenant and proposal identifier.
pub async fn lock_consolidation_proposal<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	proposal_id: Uuid,
) -> Result<Option<ConsolidationProposal>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationProposal>(
		"\
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
LIMIT 1
FOR UPDATE",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(proposal_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

/// Lists consolidation proposals for one tenant and project.
pub async fn list_consolidation_proposals<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	run_id: Option<Uuid>,
	review_state: Option<&str>,
	limit: i64,
) -> Result<Vec<ConsolidationProposal>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, ConsolidationProposal>(
		"\
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
WHERE tenant_id = $1
	AND project_id = $2
	AND ($3::uuid IS NULL OR run_id = $3)
	AND ($4::text IS NULL OR review_state = $4)
ORDER BY created_at DESC, proposal_id DESC
LIMIT $5",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(run_id)
	.bind(review_state)
	.bind(limit)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
