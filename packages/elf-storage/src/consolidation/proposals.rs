use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
	Result,
	consolidation::{
		sql::CONSOLIDATION_PROPOSAL_SELECT,
		types::{
			ConsolidationProposalReviewEventInsert, ConsolidationProposalReviewUpdate,
			ConsolidationProposalTargetRefUpdate,
		},
	},
	models::{ConsolidationProposal, ConsolidationProposalReviewEvent},
};

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
