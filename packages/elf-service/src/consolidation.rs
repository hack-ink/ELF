//! Fixture-driven consolidation run and proposal service APIs.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_domain::consolidation::{
	self, CONSOLIDATION_CONTRACT_SCHEMA_V1, ConsolidationApplyIntent, ConsolidationInputRef,
	ConsolidationLineage, ConsolidationMarkers, ConsolidationProposalContract,
	ConsolidationProposalDiff, ConsolidationReviewState, ConsolidationRunState,
	ConsolidationValidationError,
};
use elf_storage::{
	consolidation::{ConsolidationProposalReviewUpdate, ConsolidationRunStateUpdate},
	models::{ConsolidationProposal, ConsolidationRun},
};

const DEFAULT_LIST_LIMIT: i64 = 50;
const MAX_LIST_LIMIT: i64 = 200;

/// Request to create a fixture-backed consolidation run.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationRunCreateRequest {
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Agent registering the run.
	pub agent_id: String,
	/// Job kind, such as `fixture`, `manual`, or `scheduled`.
	pub job_kind: String,
	/// Input references considered by the run.
	pub input_refs: Vec<ConsolidationInputRef>,
	#[serde(default)]
	/// Aggregate source snapshot metadata for the run.
	pub source_snapshot: Value,
	/// Run lineage.
	pub lineage: ConsolidationLineage,
	#[serde(default)]
	/// Fixture-generated proposals to persist with this run.
	pub proposals: Vec<ConsolidationProposalInput>,
}

/// Fixture proposal input for a consolidation run.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalInput {
	/// Proposal kind, such as `derived_note` or `knowledge_page`.
	pub proposal_kind: String,
	/// Derived-output apply intent.
	pub apply_intent: ConsolidationApplyIntent,
	/// Source references directly supporting the proposal.
	pub source_refs: Vec<ConsolidationInputRef>,
	#[serde(default)]
	/// Aggregate source snapshot metadata for reviewer inspection.
	pub source_snapshot: Value,
	/// Proposal lineage.
	pub lineage: ConsolidationLineage,
	/// Fixture confidence in the proposal.
	pub confidence: f32,
	#[serde(default)]
	/// Review markers for contradiction and staleness checks.
	pub markers: ConsolidationMarkers,
	/// Reviewable derived-output diff.
	pub diff: ConsolidationProposalDiff,
	#[serde(default)]
	/// Derived target reference, when the target already exists.
	pub target_ref: Value,
	#[serde(default)]
	/// Proposed derived output payload.
	pub proposed_payload: Value,
}
impl ConsolidationProposalInput {
	fn validate(&self) -> Result<()> {
		let contract = ConsolidationProposalContract {
			proposal_kind: self.proposal_kind.clone(),
			apply_intent: self.apply_intent,
			source_refs: self.source_refs.clone(),
			source_snapshot: self.source_snapshot.clone(),
			lineage: self.lineage.clone(),
			confidence: self.confidence,
			markers: self.markers.clone(),
			diff: self.diff.clone(),
			target_ref: self.target_ref.clone(),
			proposed_payload: self.proposed_payload.clone(),
		};

		contract.validate().map_err(validation_error)
	}
}

/// Response returned after creating one consolidation run.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationRunCreateResponse {
	/// Created run.
	pub run: ConsolidationRunResponse,
	/// Proposals stored with the run.
	pub proposals: Vec<ConsolidationProposalResponse>,
}

/// Request to get one consolidation run.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationRunGetRequest {
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Run identifier.
	pub run_id: Uuid,
}

/// Request to list consolidation runs.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationRunsListRequest {
	/// Tenant that owns the runs.
	pub tenant_id: String,
	/// Project that owns the runs.
	pub project_id: String,
	/// Maximum number of runs to return.
	pub limit: Option<u32>,
}

/// Response returned by consolidation run listing.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationRunsListResponse {
	/// Returned runs.
	pub runs: Vec<ConsolidationRunResponse>,
}

/// Public consolidation run DTO.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationRunResponse {
	/// Consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the run.
	pub tenant_id: String,
	/// Project that owns the run.
	pub project_id: String,
	/// Agent that registered the run.
	pub agent_id: String,
	/// Versioned consolidation contract schema.
	pub contract_schema: String,
	/// Job kind, such as fixture, manual, or scheduled.
	pub job_kind: String,
	/// Current run state.
	pub status: String,
	/// Serialized input references.
	pub input_refs: Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: Value,
	/// Serialized run lineage.
	pub lineage: Value,
	/// Structured error payload for failed runs.
	pub error: Value,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Completion timestamp for terminal runs.
	pub completed_at: Option<OffsetDateTime>,
}
impl From<ConsolidationRun> for ConsolidationRunResponse {
	fn from(run: ConsolidationRun) -> Self {
		Self {
			run_id: run.run_id,
			tenant_id: run.tenant_id,
			project_id: run.project_id,
			agent_id: run.agent_id,
			contract_schema: run.contract_schema,
			job_kind: run.job_kind,
			status: run.status,
			input_refs: run.input_refs,
			source_snapshot: run.source_snapshot,
			lineage: run.lineage,
			error: run.error,
			created_at: run.created_at,
			updated_at: run.updated_at,
			completed_at: run.completed_at,
		}
	}
}

/// Request to get one consolidation proposal.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalGetRequest {
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Proposal identifier.
	pub proposal_id: Uuid,
}

/// Request to list consolidation proposals.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalsListRequest {
	/// Tenant that owns the proposals.
	pub tenant_id: String,
	/// Project that owns the proposals.
	pub project_id: String,
	/// Optional run filter.
	pub run_id: Option<Uuid>,
	/// Optional review-state filter.
	pub review_state: Option<ConsolidationReviewState>,
	/// Maximum number of proposals to return.
	pub limit: Option<u32>,
}

/// Response returned by consolidation proposal listing.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationProposalsListResponse {
	/// Returned proposals.
	pub proposals: Vec<ConsolidationProposalResponse>,
}

/// Request to transition a proposal review state.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalReviewRequest {
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent performing the review transition.
	pub reviewer_agent_id: String,
	/// Proposal identifier.
	pub proposal_id: Uuid,
	/// Requested review state.
	pub review_state: ConsolidationReviewState,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
}

/// Public consolidation proposal DTO.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationProposalResponse {
	/// Consolidation proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent that registered the proposal.
	pub agent_id: String,
	/// Versioned consolidation contract schema.
	pub contract_schema: String,
	/// Proposal kind, such as derived_note or knowledge_page.
	pub proposal_kind: String,
	/// Derived-output apply intent.
	pub apply_intent: String,
	/// Current review state.
	pub review_state: String,
	/// Serialized source references.
	pub source_refs: Value,
	/// Aggregate source snapshot metadata.
	pub source_snapshot: Value,
	/// Serialized proposal lineage.
	pub lineage: Value,
	/// Serialized reviewable diff.
	pub diff: Value,
	/// Proposal confidence score.
	pub confidence: f32,
	/// Serialized contradiction markers.
	pub contradiction_markers: Value,
	/// Serialized staleness markers.
	pub staleness_markers: Value,
	/// Serialized derived target reference.
	pub target_ref: Value,
	/// Serialized proposed derived output payload.
	pub proposed_payload: Value,
	/// Agent that last reviewed the proposal.
	pub reviewer_agent_id: Option<String>,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
	/// Timestamp of the last review transition.
	pub reviewed_at: Option<OffsetDateTime>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
impl From<ConsolidationProposal> for ConsolidationProposalResponse {
	fn from(proposal: ConsolidationProposal) -> Self {
		Self {
			proposal_id: proposal.proposal_id,
			run_id: proposal.run_id,
			tenant_id: proposal.tenant_id,
			project_id: proposal.project_id,
			agent_id: proposal.agent_id,
			contract_schema: proposal.contract_schema,
			proposal_kind: proposal.proposal_kind,
			apply_intent: proposal.apply_intent,
			review_state: proposal.review_state,
			source_refs: proposal.source_refs,
			source_snapshot: proposal.source_snapshot,
			lineage: proposal.lineage,
			diff: proposal.diff,
			confidence: proposal.confidence,
			contradiction_markers: proposal.contradiction_markers,
			staleness_markers: proposal.staleness_markers,
			target_ref: proposal.target_ref,
			proposed_payload: proposal.proposed_payload,
			reviewer_agent_id: proposal.reviewer_agent_id,
			review_comment: proposal.review_comment,
			reviewed_at: proposal.reviewed_at,
			created_at: proposal.created_at,
			updated_at: proposal.updated_at,
		}
	}
}

impl ElfService {
	/// Creates a fixture-backed consolidation run and optional proposals.
	pub async fn consolidation_run_create(
		&self,
		req: ConsolidationRunCreateRequest,
	) -> Result<ConsolidationRunCreateResponse> {
		validate_context(req.tenant_id.as_str(), req.project_id.as_str(), req.agent_id.as_str())?;
		validate_job_kind(req.job_kind.as_str())?;

		consolidation::validate_source_refs(&req.input_refs).map_err(validation_error)?;

		validate_object("source_snapshot", &req.source_snapshot)?;

		req.lineage.validate().map_err(validation_error)?;

		for proposal in &req.proposals {
			proposal.validate()?;
		}

		let has_proposals = !req.proposals.is_empty();
		let now = OffsetDateTime::now_utc();
		let run_state = if has_proposals {
			ConsolidationRunState::Running
		} else {
			ConsolidationRunState::Pending
		};
		let run_id = Uuid::new_v4();
		let mut run = ConsolidationRun {
			run_id,
			tenant_id: req.tenant_id.clone(),
			project_id: req.project_id.clone(),
			agent_id: req.agent_id.clone(),
			contract_schema: CONSOLIDATION_CONTRACT_SCHEMA_V1.to_string(),
			job_kind: req.job_kind,
			status: run_state.as_str().to_string(),
			input_refs: to_value(&req.input_refs)?,
			source_snapshot: req.source_snapshot,
			lineage: to_value(&req.lineage)?,
			error: empty_object(),
			created_at: now,
			updated_at: now,
			completed_at: terminal_time(run_state, now),
		};
		let mut proposals = Vec::with_capacity(req.proposals.len());
		let mut tx = self.db.pool.begin().await?;

		elf_storage::consolidation::insert_consolidation_run(&mut *tx, &run).await?;

		for input in req.proposals {
			let proposal = proposal_row_from_input(
				run_id,
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				req.agent_id.as_str(),
				now,
				input,
			)?;

			elf_storage::consolidation::insert_consolidation_proposal(&mut *tx, &proposal).await?;

			proposals.push(ConsolidationProposalResponse::from(proposal));
		}

		if has_proposals {
			run_state
				.validate_transition(ConsolidationRunState::Completed)
				.map_err(validation_error)?;

			let terminal_error = empty_object();

			run = elf_storage::consolidation::update_consolidation_run_state(
				&mut *tx,
				ConsolidationRunStateUpdate {
					tenant_id: req.tenant_id.as_str(),
					project_id: req.project_id.as_str(),
					run_id,
					status: ConsolidationRunState::Completed.as_str(),
					error: &terminal_error,
					now,
				},
			)
			.await?
			.ok_or_else(|| Error::NotFound {
				message: "consolidation run not found".to_string(),
			})?;
		}

		tx.commit().await?;

		Ok(ConsolidationRunCreateResponse { run: ConsolidationRunResponse::from(run), proposals })
	}

	/// Fetches one consolidation run.
	pub async fn consolidation_run_get(
		&self,
		req: ConsolidationRunGetRequest,
	) -> Result<ConsolidationRunResponse> {
		let run = elf_storage::consolidation::get_consolidation_run(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.run_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound { message: "consolidation run not found".to_string() })?;

		Ok(ConsolidationRunResponse::from(run))
	}

	/// Lists consolidation runs.
	pub async fn consolidation_runs_list(
		&self,
		req: ConsolidationRunsListRequest,
	) -> Result<ConsolidationRunsListResponse> {
		let limit = bounded_limit(req.limit);
		let rows = elf_storage::consolidation::list_consolidation_runs(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			limit,
		)
		.await?;
		let runs = rows.into_iter().map(ConsolidationRunResponse::from).collect();

		Ok(ConsolidationRunsListResponse { runs })
	}

	/// Fetches one consolidation proposal.
	pub async fn consolidation_proposal_get(
		&self,
		req: ConsolidationProposalGetRequest,
	) -> Result<ConsolidationProposalResponse> {
		let proposal = elf_storage::consolidation::get_consolidation_proposal(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.proposal_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound {
			message: "consolidation proposal not found".to_string(),
		})?;

		Ok(ConsolidationProposalResponse::from(proposal))
	}

	/// Lists consolidation proposals.
	pub async fn consolidation_proposals_list(
		&self,
		req: ConsolidationProposalsListRequest,
	) -> Result<ConsolidationProposalsListResponse> {
		let limit = bounded_limit(req.limit);
		let review_state = req.review_state.map(ConsolidationReviewState::as_str);
		let rows = elf_storage::consolidation::list_consolidation_proposals(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.run_id,
			review_state,
			limit,
		)
		.await?;
		let proposals = rows.into_iter().map(ConsolidationProposalResponse::from).collect();

		Ok(ConsolidationProposalsListResponse { proposals })
	}

	/// Applies one allowed proposal review-state transition.
	pub async fn consolidation_proposal_review(
		&self,
		req: ConsolidationProposalReviewRequest,
	) -> Result<ConsolidationProposalResponse> {
		validate_context(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.reviewer_agent_id.as_str(),
		)?;

		let existing = elf_storage::consolidation::get_consolidation_proposal(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.proposal_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound {
			message: "consolidation proposal not found".to_string(),
		})?;
		let current =
			ConsolidationReviewState::parse(existing.review_state.as_str()).ok_or_else(|| {
				Error::InvalidRequest {
					message: "stored proposal review_state is invalid".to_string(),
				}
			})?;

		current.validate_transition(req.review_state).map_err(validation_error)?;

		let updated = elf_storage::consolidation::update_consolidation_proposal_review(
			&self.db.pool,
			ConsolidationProposalReviewUpdate {
				tenant_id: req.tenant_id.as_str(),
				project_id: req.project_id.as_str(),
				proposal_id: req.proposal_id,
				review_state: req.review_state.as_str(),
				reviewer_agent_id: req.reviewer_agent_id.as_str(),
				review_comment: req.review_comment.as_deref(),
				now: OffsetDateTime::now_utc(),
			},
		)
		.await?
		.ok_or_else(|| Error::NotFound {
			message: "consolidation proposal not found".to_string(),
		})?;

		Ok(ConsolidationProposalResponse::from(updated))
	}
}

fn proposal_row_from_input(
	run_id: Uuid,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	now: OffsetDateTime,
	input: ConsolidationProposalInput,
) -> Result<ConsolidationProposal> {
	Ok(ConsolidationProposal {
		proposal_id: Uuid::new_v4(),
		run_id,
		tenant_id: tenant_id.to_string(),
		project_id: project_id.to_string(),
		agent_id: agent_id.to_string(),
		contract_schema: CONSOLIDATION_CONTRACT_SCHEMA_V1.to_string(),
		proposal_kind: input.proposal_kind,
		apply_intent: input.apply_intent.as_str().to_string(),
		review_state: ConsolidationReviewState::Proposed.as_str().to_string(),
		source_refs: to_value(&input.source_refs)?,
		source_snapshot: input.source_snapshot,
		lineage: to_value(&input.lineage)?,
		diff: to_value(&input.diff)?,
		confidence: input.confidence,
		contradiction_markers: to_value(&input.markers.contradictions)?,
		staleness_markers: to_value(&input.markers.staleness)?,
		target_ref: input.target_ref,
		proposed_payload: input.proposed_payload,
		reviewer_agent_id: None,
		review_comment: None,
		reviewed_at: None,
		created_at: now,
		updated_at: now,
	})
}

fn validate_context(tenant_id: &str, project_id: &str, agent_id: &str) -> Result<()> {
	validate_non_empty("tenant_id", tenant_id)?;
	validate_non_empty("project_id", project_id)?;

	validate_non_empty("agent_id", agent_id)
}

fn validate_job_kind(job_kind: &str) -> Result<()> {
	validate_non_empty("job_kind", job_kind)
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<()> {
	if value.trim().is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} must not be empty.") });
	}

	Ok(())
}

fn validate_object(field: &str, value: &Value) -> Result<()> {
	if matches!(value, Value::Object(_)) {
		Ok(())
	} else {
		Err(Error::InvalidRequest { message: format!("{field} must be a JSON object.") })
	}
}

fn validation_error(err: ConsolidationValidationError) -> Error {
	Error::InvalidRequest { message: err.to_string() }
}

fn bounded_limit(limit: Option<u32>) -> i64 {
	limit.map(i64::from).unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, MAX_LIST_LIMIT)
}

fn to_value<T>(value: &T) -> Result<Value>
where
	T: Serialize,
{
	serde_json::to_value(value).map_err(|err| Error::InvalidRequest {
		message: format!("failed to serialize consolidation contract: {err}"),
	})
}

fn empty_object() -> Value {
	Value::Object(Map::new())
}

fn terminal_time(state: ConsolidationRunState, now: OffsetDateTime) -> Option<OffsetDateTime> {
	match state {
		ConsolidationRunState::Completed
		| ConsolidationRunState::Failed
		| ConsolidationRunState::Cancelled => Some(now),
		ConsolidationRunState::Pending | ConsolidationRunState::Running => None,
	}
}
