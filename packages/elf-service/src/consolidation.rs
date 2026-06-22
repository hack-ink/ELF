//! Fixture-driven consolidation run and proposal service APIs.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{Postgres, Transaction};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{ElfService, Error, InsertVersionArgs, Result};
use elf_config::Config;
use elf_domain::{
	consolidation::{
		self, CONSOLIDATION_CONTRACT_SCHEMA_V1, ConsolidationApplyIntent, ConsolidationInputRef,
		ConsolidationJobPayload, ConsolidationLineage, ConsolidationMarkers,
		ConsolidationProposalContract, ConsolidationProposalDiff, ConsolidationReviewAction,
		ConsolidationReviewState, ConsolidationRunState, ConsolidationUnsupportedClaimFlag,
		ConsolidationValidationError,
	},
	ttl,
	writegate::{self, NoteInput},
};
use elf_storage::{
	consolidation::{
		ConsolidationProposalReviewEventInsert, ConsolidationProposalReviewUpdate,
		ConsolidationProposalTargetRefUpdate, ConsolidationRunJobInsert,
	},
	models::{
		ConsolidationProposal, ConsolidationProposalReviewEvent, ConsolidationRun, MemoryNote,
	},
	queries,
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
	/// Job kind, such as `fixture` or `manual`.
	pub job_kind: String,
	/// Input references considered by the run.
	pub input_refs: Vec<ConsolidationInputRef>,
	#[serde(default = "empty_object")]
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
	#[serde(default = "empty_object")]
	/// Aggregate source snapshot metadata for reviewer inspection.
	pub source_snapshot: Value,
	/// Proposal lineage.
	pub lineage: ConsolidationLineage,
	/// Fixture confidence in the proposal.
	pub confidence: f32,
	#[serde(default)]
	/// Unsupported claims reviewers must inspect before accepting the proposal.
	pub unsupported_claim_flags: Vec<ConsolidationUnsupportedClaimFlag>,
	#[serde(default)]
	/// Review markers for contradiction and staleness checks.
	pub markers: ConsolidationMarkers,
	/// Reviewable derived-output diff.
	pub diff: ConsolidationProposalDiff,
	#[serde(default = "empty_object")]
	/// Derived target reference, when the target already exists.
	pub target_ref: Value,
	#[serde(default = "empty_object")]
	/// Proposed derived output payload.
	pub proposed_payload: Value,
}
impl ConsolidationProposalInput {
	fn into_contract(self) -> ConsolidationProposalContract {
		ConsolidationProposalContract {
			proposal_kind: self.proposal_kind,
			apply_intent: self.apply_intent,
			source_refs: self.source_refs,
			source_snapshot: self.source_snapshot,
			lineage: self.lineage,
			confidence: self.confidence,
			unsupported_claim_flags: self.unsupported_claim_flags,
			markers: self.markers,
			diff: self.diff,
			target_ref: self.target_ref,
			proposed_payload: self.proposed_payload,
		}
	}
}

/// Response returned after creating one consolidation run.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationRunCreateResponse {
	/// Created run.
	pub run: ConsolidationRunResponse,
	/// Enqueued worker job identifier.
	pub job_id: Uuid,
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
	/// Job kind, such as fixture or manual.
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

/// Request to apply one proposal review action.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalReviewRequest {
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent performing the review action.
	pub reviewer_agent_id: String,
	/// Proposal identifier.
	pub proposal_id: Uuid,
	/// Requested review action.
	pub review_action: ConsolidationReviewAction,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
}

/// Public consolidation proposal review audit DTO.
#[derive(Clone, Debug, Serialize)]
pub struct ConsolidationProposalReviewEventResponse {
	/// Review event identifier.
	pub review_id: Uuid,
	/// Reviewed proposal identifier.
	pub proposal_id: Uuid,
	/// Parent consolidation run identifier.
	pub run_id: Uuid,
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent that performed the review action.
	pub reviewer_agent_id: String,
	/// Review action requested by the reviewer.
	pub action: String,
	/// Review state before the transition.
	pub from_review_state: String,
	/// Review state after the transition.
	pub to_review_state: String,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}
impl From<ConsolidationProposalReviewEvent> for ConsolidationProposalReviewEventResponse {
	fn from(event: ConsolidationProposalReviewEvent) -> Self {
		Self {
			review_id: event.review_id,
			proposal_id: event.proposal_id,
			run_id: event.run_id,
			tenant_id: event.tenant_id,
			project_id: event.project_id,
			reviewer_agent_id: event.reviewer_agent_id,
			action: event.action,
			from_review_state: event.from_review_state,
			to_review_state: event.to_review_state,
			review_comment: event.review_comment,
			created_at: event.created_at,
		}
	}
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
	/// Serialized unsupported-claim flags.
	pub unsupported_claim_flags: Value,
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
	/// Append-only review events for detail readback.
	pub review_events: Vec<ConsolidationProposalReviewEventResponse>,
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
			unsupported_claim_flags: proposal.unsupported_claim_flags,
			contradiction_markers: proposal.contradiction_markers,
			staleness_markers: proposal.staleness_markers,
			target_ref: proposal.target_ref,
			proposed_payload: proposal.proposed_payload,
			reviewer_agent_id: proposal.reviewer_agent_id,
			review_comment: proposal.review_comment,
			reviewed_at: proposal.reviewed_at,
			created_at: proposal.created_at,
			updated_at: proposal.updated_at,
			review_events: Vec::new(),
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
struct PromotedMemoryPayload {
	#[serde(rename = "type")]
	note_type: String,
	text: String,
	scope: Option<String>,
	key: Option<String>,
	importance: Option<f32>,
	confidence: Option<f32>,
	ttl_days: Option<i64>,
	#[serde(default = "empty_object")]
	source_ref: Value,
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

		let proposal_contracts =
			req.proposals.into_iter().map(ConsolidationProposalInput::into_contract).collect();
		let payload = ConsolidationJobPayload {
			contract_schema: CONSOLIDATION_CONTRACT_SCHEMA_V1.to_string(),
			proposals: proposal_contracts,
		};

		payload.validate().map_err(validation_error)?;

		let now = OffsetDateTime::now_utc();
		let run_state = ConsolidationRunState::Pending;
		let run_id = Uuid::new_v4();
		let job_id = Uuid::new_v4();
		let run = ConsolidationRun {
			run_id,
			tenant_id: req.tenant_id.clone(),
			project_id: req.project_id.clone(),
			agent_id: req.agent_id.clone(),
			contract_schema: CONSOLIDATION_CONTRACT_SCHEMA_V1.to_string(),
			job_kind: req.job_kind.clone(),
			status: run_state.as_str().to_string(),
			input_refs: to_value(&req.input_refs)?,
			source_snapshot: req.source_snapshot,
			lineage: to_value(&req.lineage)?,
			error: empty_object(),
			created_at: now,
			updated_at: now,
			completed_at: terminal_time(run_state, now),
		};
		let payload_value = to_value(&payload)?;
		let mut tx = self.db.pool.begin().await?;

		elf_storage::consolidation::insert_consolidation_run(&mut *tx, &run).await?;
		elf_storage::consolidation::insert_consolidation_run_job(
			&mut *tx,
			ConsolidationRunJobInsert {
				job_id,
				run_id,
				tenant_id: req.tenant_id.as_str(),
				project_id: req.project_id.as_str(),
				agent_id: req.agent_id.as_str(),
				job_kind: req.job_kind.as_str(),
				payload: &payload_value,
				now,
			},
		)
		.await?;

		tx.commit().await?;

		Ok(ConsolidationRunCreateResponse {
			run: ConsolidationRunResponse::from(run),
			job_id,
			proposals: Vec::new(),
		})
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
		let review_events = self
			.consolidation_proposal_review_events(
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				req.proposal_id,
			)
			.await?;
		let mut response = ConsolidationProposalResponse::from(proposal);

		response.review_events = review_events;

		Ok(response)
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

	/// Applies one allowed proposal review action.
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
		let now = OffsetDateTime::now_utc();
		let steps = review_steps(current, req.review_action)?;
		let mut tx = self.db.pool.begin().await?;
		let mut last_state = current;
		let mut updated = existing;

		for (step_index, (action, next_state)) in steps.into_iter().enumerate() {
			last_state.validate_transition(next_state).map_err(validation_error)?;

			let transition_time = now.saturating_add(Duration::milliseconds(step_index as i64));

			elf_storage::consolidation::insert_consolidation_proposal_review_event(
				&mut *tx,
				ConsolidationProposalReviewEventInsert {
					review_id: Uuid::new_v4(),
					proposal_id: req.proposal_id,
					run_id: updated.run_id,
					tenant_id: req.tenant_id.as_str(),
					project_id: req.project_id.as_str(),
					reviewer_agent_id: req.reviewer_agent_id.as_str(),
					action: action.as_str(),
					from_review_state: last_state.as_str(),
					to_review_state: next_state.as_str(),
					review_comment: req.review_comment.as_deref(),
					created_at: transition_time,
				},
			)
			.await?;

			updated = elf_storage::consolidation::update_consolidation_proposal_review(
				&mut *tx,
				ConsolidationProposalReviewUpdate {
					tenant_id: req.tenant_id.as_str(),
					project_id: req.project_id.as_str(),
					proposal_id: req.proposal_id,
					review_state: next_state.as_str(),
					reviewer_agent_id: req.reviewer_agent_id.as_str(),
					review_comment: req.review_comment.as_deref(),
					now: transition_time,
				},
			)
			.await?
			.ok_or_else(|| Error::NotFound {
				message: "consolidation proposal not found".to_string(),
			})?;

			if action == ConsolidationReviewAction::Apply {
				updated = self
					.apply_consolidation_proposal_to_memory(
						&mut tx,
						updated,
						req.reviewer_agent_id.as_str(),
						req.review_comment.as_deref(),
						transition_time,
					)
					.await?;
			}

			last_state = next_state;
		}

		tx.commit().await?;

		let review_events = self
			.consolidation_proposal_review_events(
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				req.proposal_id,
			)
			.await?;
		let mut response = ConsolidationProposalResponse::from(updated);

		response.review_events = review_events;

		Ok(response)
	}

	async fn apply_consolidation_proposal_to_memory(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		proposal: ConsolidationProposal,
		reviewer_agent_id: &str,
		review_comment: Option<&str>,
		now: OffsetDateTime,
	) -> Result<ConsolidationProposal> {
		let note_id = match proposal.apply_intent.as_str() {
			"create_derived_note" =>
				create_promoted_memory_note(
					tx,
					&proposal,
					reviewer_agent_id,
					review_comment,
					&self.cfg,
					now,
				)
				.await?,
			"update_derived_note" =>
				update_promoted_memory_note(
					tx,
					&proposal,
					reviewer_agent_id,
					review_comment,
					&self.cfg,
					now,
				)
				.await?,
			_ => return Ok(proposal),
		};
		let target_ref = promoted_memory_target_ref(note_id, now);

		elf_storage::consolidation::update_consolidation_proposal_target_ref(
			&mut **tx,
			ConsolidationProposalTargetRefUpdate {
				tenant_id: proposal.tenant_id.as_str(),
				project_id: proposal.project_id.as_str(),
				proposal_id: proposal.proposal_id,
				target_ref: &target_ref,
				now,
			},
		)
		.await?
		.ok_or_else(|| Error::NotFound { message: "consolidation proposal not found".to_string() })
	}

	async fn consolidation_proposal_review_events(
		&self,
		tenant_id: &str,
		project_id: &str,
		proposal_id: Uuid,
	) -> Result<Vec<ConsolidationProposalReviewEventResponse>> {
		let events = elf_storage::consolidation::list_consolidation_proposal_review_events(
			&self.db.pool,
			tenant_id,
			project_id,
			proposal_id,
		)
		.await?;

		Ok(events.into_iter().map(ConsolidationProposalReviewEventResponse::from).collect())
	}
}

fn validate_context(tenant_id: &str, project_id: &str, agent_id: &str) -> Result<()> {
	validate_non_empty("tenant_id", tenant_id)?;
	validate_non_empty("project_id", project_id)?;

	validate_non_empty("agent_id", agent_id)
}

fn validate_job_kind(job_kind: &str) -> Result<()> {
	validate_non_empty("job_kind", job_kind)?;

	match job_kind {
		"fixture" | "manual" => Ok(()),
		_ => Err(Error::InvalidRequest {
			message: "job_kind must be fixture or manual for consolidation v1.".to_string(),
		}),
	}
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

fn review_steps(
	current: ConsolidationReviewState,
	action: ConsolidationReviewAction,
) -> Result<Vec<(ConsolidationReviewAction, ConsolidationReviewState)>> {
	let steps = match action {
		ConsolidationReviewAction::Approve =>
			vec![(ConsolidationReviewAction::Approve, ConsolidationReviewState::Approved)],
		ConsolidationReviewAction::Apply => match current {
			ConsolidationReviewState::Proposed => vec![
				(ConsolidationReviewAction::Approve, ConsolidationReviewState::Approved),
				(ConsolidationReviewAction::Apply, ConsolidationReviewState::Applied),
			],
			ConsolidationReviewState::Approved =>
				vec![(ConsolidationReviewAction::Apply, ConsolidationReviewState::Applied)],
			ConsolidationReviewState::Rejected
			| ConsolidationReviewState::Applied
			| ConsolidationReviewState::Archived =>
				vec![(ConsolidationReviewAction::Apply, ConsolidationReviewState::Applied)],
		},
		ConsolidationReviewAction::Discard =>
			vec![(ConsolidationReviewAction::Discard, ConsolidationReviewState::Rejected)],
		ConsolidationReviewAction::Defer =>
			vec![(ConsolidationReviewAction::Defer, ConsolidationReviewState::Archived)],
	};
	let mut state = current;

	for (_, next_state) in &steps {
		state.validate_transition(*next_state).map_err(validation_error)?;

		state = *next_state;
	}

	Ok(steps)
}

fn promoted_memory_payload(
	proposal: &ConsolidationProposal,
	cfg: &Config,
) -> Result<PromotedMemoryPayload> {
	let payload: PromotedMemoryPayload = serde_json::from_value(proposal.proposed_payload.clone())
		.map_err(|err| Error::InvalidRequest {
			message: format!("proposed_payload is not a memory note payload: {err}"),
		})?;
	let scope = payload.scope.as_deref().unwrap_or("agent_private");
	let gate = NoteInput {
		note_type: payload.note_type.clone(),
		scope: scope.to_string(),
		text: payload.text.clone(),
	};

	if let Err(code) = writegate::writegate(&gate, cfg) {
		return Err(Error::InvalidRequest {
			message: format!(
				"proposed memory failed writegate: {}",
				crate::writegate_reason_code(code)
			),
		});
	}

	if !matches!(payload.source_ref, Value::Object(_)) {
		return Err(Error::InvalidRequest {
			message: "proposed_payload.source_ref must be a JSON object when provided.".to_string(),
		});
	}
	if payload.importance.is_some_and(invalid_score)
		|| payload.confidence.is_some_and(invalid_score)
	{
		return Err(Error::InvalidRequest {
			message: "proposed memory scores must be finite values in 0.0..=1.0.".to_string(),
		});
	}

	Ok(payload)
}

fn invalid_score(score: f32) -> bool {
	!score.is_finite() || !(0.0..=1.0).contains(&score)
}

fn target_note_id(proposal: &ConsolidationProposal) -> Result<Uuid> {
	let raw = proposal
		.target_ref
		.get("id")
		.or_else(|| proposal.target_ref.get("note_id"))
		.and_then(Value::as_str)
		.ok_or_else(|| Error::InvalidRequest {
			message: "update_derived_note requires target_ref.id or target_ref.note_id."
				.to_string(),
		})?;

	Uuid::parse_str(raw).map_err(|err| Error::InvalidRequest {
		message: format!("target_ref note id is invalid: {err}"),
	})
}

fn normalized_optional_string(value: Option<String>) -> Option<String> {
	value.map(|raw| raw.trim().to_string()).filter(|trimmed| !trimmed.is_empty())
}

fn promotion_source_ref(
	proposal: &ConsolidationProposal,
	proposed_source_ref: &Value,
	reviewer_agent_id: &str,
	review_comment: Option<&str>,
	now: OffsetDateTime,
) -> Value {
	serde_json::json!({
		"schema": "elf.memory_promotion/v1",
		"proposal_id": proposal.proposal_id,
		"run_id": proposal.run_id,
		"proposal_kind": proposal.proposal_kind,
		"apply_intent": proposal.apply_intent,
		"source_refs": proposal.source_refs,
		"source_snapshot": proposal.source_snapshot,
		"lineage": proposal.lineage,
		"unsupported_claim_flags": proposal.unsupported_claim_flags,
		"review": {
			"action": "apply",
			"reviewer_agent_id": reviewer_agent_id,
			"review_comment": review_comment,
			"applied_at": now,
		},
		"proposed_source_ref": proposed_source_ref,
	})
}

fn promoted_memory_target_ref(note_id: Uuid, now: OffsetDateTime) -> Value {
	serde_json::json!({
		"schema": "elf.memory_record_ref/v1",
		"kind": "note",
		"id": note_id,
		"status": "active",
		"applied_at": now,
	})
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

async fn create_promoted_memory_note(
	tx: &mut Transaction<'_, Postgres>,
	proposal: &ConsolidationProposal,
	reviewer_agent_id: &str,
	review_comment: Option<&str>,
	cfg: &Config,
	now: OffsetDateTime,
) -> Result<Uuid> {
	let payload = promoted_memory_payload(proposal, cfg)?;
	let scope = payload.scope.clone().unwrap_or_else(|| "agent_private".to_string());
	let note_type = payload.note_type;
	let expires_at = ttl::compute_expires_at(payload.ttl_days, &note_type, cfg, now);
	let source_ref =
		promotion_source_ref(proposal, &payload.source_ref, reviewer_agent_id, review_comment, now);
	let note_id = Uuid::new_v4();
	let note = MemoryNote {
		note_id,
		tenant_id: proposal.tenant_id.clone(),
		project_id: proposal.project_id.clone(),
		agent_id: reviewer_agent_id.to_string(),
		scope,
		r#type: note_type,
		key: normalized_optional_string(payload.key),
		text: payload.text,
		importance: payload.importance.unwrap_or(proposal.confidence),
		confidence: payload.confidence.unwrap_or(proposal.confidence),
		status: "active".to_string(),
		created_at: now,
		updated_at: now,
		expires_at,
		embedding_version: crate::embedding_version(cfg),
		source_ref,
		hit_count: 0,
		last_hit_at: None,
	};

	queries::insert_note(&mut **tx, &note).await?;
	crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id,
			op: "ADD",
			prev_snapshot: None,
			new_snapshot: Some(crate::note_snapshot(&note)),
			reason: "consolidation_apply.create_derived_note",
			actor: reviewer_agent_id,
			ts: now,
		},
	)
	.await?;
	crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", &note.embedding_version, now).await?;

	Ok(note_id)
}

async fn update_promoted_memory_note(
	tx: &mut Transaction<'_, Postgres>,
	proposal: &ConsolidationProposal,
	reviewer_agent_id: &str,
	review_comment: Option<&str>,
	cfg: &Config,
	now: OffsetDateTime,
) -> Result<Uuid> {
	let payload = promoted_memory_payload(proposal, cfg)?;
	let note_id = target_note_id(proposal)?;
	let mut note = sqlx::query_as::<_, MemoryNote>(
		"\
SELECT *
FROM memory_notes
WHERE note_id = $1 AND tenant_id = $2 AND project_id = $3
FOR UPDATE",
	)
	.bind(note_id)
	.bind(proposal.tenant_id.as_str())
	.bind(proposal.project_id.as_str())
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::InvalidRequest {
		message: "Target memory note was not found.".to_string(),
	})?;

	if note.status != "active" {
		return Err(Error::InvalidRequest {
			message: "Only active target memory can be updated by proposal apply.".to_string(),
		});
	}

	let prev_snapshot = crate::note_snapshot(&note);

	note.scope = payload.scope.unwrap_or(note.scope);
	note.r#type = payload.note_type;
	note.key = normalized_optional_string(payload.key);
	note.text = payload.text;
	note.importance = payload.importance.unwrap_or(note.importance);
	note.confidence = payload.confidence.unwrap_or(note.confidence);

	if payload.ttl_days.is_some() {
		note.expires_at = ttl::compute_expires_at(payload.ttl_days, &note.r#type, cfg, now);
	}

	note.updated_at = now;
	note.source_ref =
		promotion_source_ref(proposal, &payload.source_ref, reviewer_agent_id, review_comment, now);

	update_promoted_note_row(tx, &note).await?;

	crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id,
			op: "UPDATE",
			prev_snapshot: Some(prev_snapshot),
			new_snapshot: Some(crate::note_snapshot(&note)),
			reason: "consolidation_apply.update_derived_note",
			actor: reviewer_agent_id,
			ts: now,
		},
	)
	.await?;
	crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", &note.embedding_version, now).await?;

	Ok(note_id)
}

async fn update_promoted_note_row(
	tx: &mut Transaction<'_, Postgres>,
	note: &MemoryNote,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE memory_notes
SET
	scope = $1,
	type = $2,
	key = $3,
	text = $4,
	importance = $5,
	confidence = $6,
	updated_at = $7,
	expires_at = $8,
	source_ref = $9
WHERE note_id = $10",
	)
	.bind(note.scope.as_str())
	.bind(note.r#type.as_str())
	.bind(note.key.as_deref())
	.bind(note.text.as_str())
	.bind(note.importance)
	.bind(note.confidence)
	.bind(note.updated_at)
	.bind(note.expires_at)
	.bind(&note.source_ref)
	.bind(note.note_id)
	.execute(&mut **tx)
	.await?;

	Ok(())
}
