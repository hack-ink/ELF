//! Consolidation run and proposal persistence queries.

use serde_json::Value;
use sqlx::PgExecutor;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	Result,
	db::Db,
	models::{
		ConsolidationProposal, ConsolidationProposalReviewEvent, ConsolidationRun,
		ConsolidationRunJob,
	},
};

const CONSOLIDATION_RUN_SELECT: &str = "\
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
const CONSOLIDATION_PROPOSAL_SELECT: &str = "\
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

/// Arguments for updating a consolidation run state.
pub struct ConsolidationRunStateUpdate<'a> {
	/// Tenant that owns the run.
	pub tenant_id: &'a str,
	/// Project that owns the run.
	pub project_id: &'a str,
	/// Run identifier.
	pub run_id: Uuid,
	/// New run status.
	pub status: &'a str,
	/// Structured error payload for terminal failure states.
	pub error: &'a Value,
	/// Update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for updating a consolidation proposal review state.
pub struct ConsolidationProposalReviewUpdate<'a> {
	/// Tenant that owns the proposal.
	pub tenant_id: &'a str,
	/// Project that owns the proposal.
	pub project_id: &'a str,
	/// Proposal identifier.
	pub proposal_id: Uuid,
	/// New review state.
	pub review_state: &'a str,
	/// Reviewing agent identifier.
	pub reviewer_agent_id: &'a str,
	/// Optional reviewer comment.
	pub review_comment: Option<&'a str>,
	/// Update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for updating a consolidation proposal target reference.
pub struct ConsolidationProposalTargetRefUpdate<'a> {
	/// Tenant that owns the proposal.
	pub tenant_id: &'a str,
	/// Project that owns the proposal.
	pub project_id: &'a str,
	/// Proposal identifier.
	pub proposal_id: Uuid,
	/// New target reference.
	pub target_ref: &'a Value,
	/// Update timestamp.
	pub now: OffsetDateTime,
}

/// Arguments for inserting a consolidation proposal review event.
pub struct ConsolidationProposalReviewEventInsert<'a> {
	/// Review event identifier.
	pub review_id: Uuid,
	/// Reviewed proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: &'a str,
	/// Project that owns the proposal.
	pub project_id: &'a str,
	/// Reviewing agent identifier.
	pub reviewer_agent_id: &'a str,
	/// Review action requested by the reviewer.
	pub action: &'a str,
	/// Review state before the transition.
	pub from_review_state: &'a str,
	/// Review state after the transition.
	pub to_review_state: &'a str,
	/// Optional reviewer comment.
	pub review_comment: Option<&'a str>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Arguments for inserting a consolidation worker job.
pub struct ConsolidationRunJobInsert<'a> {
	/// Worker job identifier.
	pub job_id: Uuid,
	/// Consolidation run to materialize.
	pub run_id: Uuid,
	/// Tenant that owns the run.
	pub tenant_id: &'a str,
	/// Project that owns the run.
	pub project_id: &'a str,
	/// Agent that registered the run.
	pub agent_id: &'a str,
	/// Job kind, such as fixture or manual.
	pub job_kind: &'a str,
	/// Queued proposal payload.
	pub payload: &'a Value,
	/// Creation timestamp.
	pub now: OffsetDateTime,
}

/// Inserts one consolidation run.
pub async fn insert_consolidation_run<'e, E>(executor: E, run: &ConsolidationRun) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO consolidation_runs (
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
	error,
	created_at,
	updated_at,
	completed_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
	)
	.bind(run.run_id)
	.bind(run.tenant_id.as_str())
	.bind(run.project_id.as_str())
	.bind(run.agent_id.as_str())
	.bind(run.contract_schema.as_str())
	.bind(run.job_kind.as_str())
	.bind(run.status.as_str())
	.bind(&run.input_refs)
	.bind(&run.source_snapshot)
	.bind(&run.lineage)
	.bind(&run.error)
	.bind(run.created_at)
	.bind(run.updated_at)
	.bind(run.completed_at)
	.execute(executor)
	.await?;

	Ok(())
}

/// Enqueues one consolidation worker job.
pub async fn insert_consolidation_run_job<'e, E>(
	executor: E,
	args: ConsolidationRunJobInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO consolidation_run_jobs (
	job_id,
	run_id,
	tenant_id,
	project_id,
	agent_id,
	job_kind,
	status,
	payload,
	available_at,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,$6,'PENDING',$7,$8,$8,$8)",
	)
	.bind(args.job_id)
	.bind(args.run_id)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.agent_id)
	.bind(args.job_kind)
	.bind(args.payload)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Fetches one consolidation run by tenant and run identifier.
pub async fn get_consolidation_run<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	run_id: Uuid,
) -> Result<Option<ConsolidationRun>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationRun>(CONSOLIDATION_RUN_SELECT)
		.bind(tenant_id)
		.bind(project_id)
		.bind(run_id)
		.fetch_optional(executor)
		.await?;

	Ok(row)
}

/// Lists consolidation runs for one tenant and project.
pub async fn list_consolidation_runs<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	limit: i64,
) -> Result<Vec<ConsolidationRun>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, ConsolidationRun>(
		"\
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
WHERE tenant_id = $1 AND project_id = $2
ORDER BY created_at DESC, run_id DESC
LIMIT $3",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(limit)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Claims the next due consolidation worker job and leases it until `lease_seconds`.
pub async fn claim_next_consolidation_run_job(
	db: &Db,
	now: OffsetDateTime,
	lease_seconds: i64,
) -> Result<Option<ConsolidationRunJob>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query_as::<_, ConsolidationRunJob>(
		"\
SELECT
	job_id,
	run_id,
	tenant_id,
	project_id,
	agent_id,
	job_kind,
	status,
	payload,
	attempts,
	last_error,
	available_at,
	created_at,
	updated_at
FROM consolidation_run_jobs
WHERE status IN ('PENDING','FAILED','CLAIMED') AND available_at <= $1
ORDER BY available_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED",
	)
	.bind(now)
	.fetch_optional(&mut *tx)
	.await?;
	let job = if let Some(mut job) = row {
		let lease_until = now + Duration::seconds(lease_seconds);

		sqlx::query(
			"\
UPDATE consolidation_run_jobs
SET status = 'CLAIMED', available_at = $1, updated_at = $2
WHERE job_id = $3",
		)
		.bind(lease_until)
		.bind(now)
		.bind(job.job_id)
		.execute(&mut *tx)
		.await?;

		job.status = "CLAIMED".to_string();
		job.available_at = lease_until;
		job.updated_at = now;

		Some(job)
	} else {
		None
	};

	tx.commit().await?;

	Ok(job)
}

/// Marks a consolidation worker job as completed.
pub async fn mark_consolidation_run_job_done<'e, E>(
	executor: E,
	job_id: Uuid,
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
UPDATE consolidation_run_jobs
SET status = 'DONE', updated_at = $1
WHERE job_id = $2",
	)
	.bind(now)
	.bind(job_id)
	.execute(executor)
	.await?;

	Ok(())
}

/// Marks a consolidation worker job as failed and schedules its retry.
pub async fn mark_consolidation_run_job_failed(
	db: &Db,
	job_id: Uuid,
	attempts: i32,
	error_text: &str,
	available_at: OffsetDateTime,
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE consolidation_run_jobs
SET status = 'FAILED',
	attempts = $1,
	last_error = $2,
	available_at = $3,
	updated_at = $4
WHERE job_id = $5",
	)
	.bind(attempts)
	.bind(error_text)
	.bind(available_at)
	.bind(now)
	.bind(job_id)
	.execute(&db.pool)
	.await?;

	Ok(())
}

/// Updates one consolidation run state.
pub async fn update_consolidation_run_state<'e, E>(
	executor: E,
	args: ConsolidationRunStateUpdate<'_>,
) -> Result<Option<ConsolidationRun>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationRun>(
		"\
UPDATE consolidation_runs
SET
	status = $1,
	error = $2,
	updated_at = $3,
	completed_at = CASE
		WHEN $1 IN ('completed', 'failed', 'cancelled') THEN $3
		ELSE completed_at
	END
WHERE tenant_id = $4 AND project_id = $5 AND run_id = $6
RETURNING
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
	completed_at",
	)
	.bind(args.status)
	.bind(args.error)
	.bind(args.now)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.run_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

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
