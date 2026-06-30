use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, knowledge::types::KnowledgeProposalSource};

/// Fetches applied proposal sources by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_proposal_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	proposal_ids: &[Uuid],
) -> Result<Vec<KnowledgeProposalSource>>
where
	E: PgExecutor<'e>,
{
	if proposal_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeProposalSource>(
		"\
SELECT
	proposal_id,
	run_id,
	agent_id,
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
	updated_at
FROM consolidation_proposals
WHERE tenant_id = $1
	AND project_id = $2
	AND proposal_id = ANY($3::uuid[])
	AND review_state = 'applied'
ORDER BY updated_at ASC, proposal_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(proposal_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
