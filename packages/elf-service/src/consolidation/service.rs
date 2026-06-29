use sqlx::{Postgres, Transaction};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	consolidation::{
		promotion::{self},
		types::{
			self, ConsolidationProposalGetRequest, ConsolidationProposalInput,
			ConsolidationProposalResponse, ConsolidationProposalReviewEventResponse,
			ConsolidationProposalReviewRequest, ConsolidationProposalsListRequest,
			ConsolidationProposalsListResponse, ConsolidationRunCreateRequest,
			ConsolidationRunCreateResponse, ConsolidationRunGetRequest, ConsolidationRunResponse,
			ConsolidationRunsListRequest, ConsolidationRunsListResponse,
		},
		validation::{self, validation_error},
	},
};
use elf_domain::consolidation::{
	self, CONSOLIDATION_CONTRACT_SCHEMA_V1, ConsolidationJobPayload, ConsolidationReviewAction,
	ConsolidationReviewState, ConsolidationRunState,
};
use elf_storage::{
	consolidation::{
		ConsolidationProposalReviewEventInsert, ConsolidationProposalReviewUpdate,
		ConsolidationProposalTargetRefUpdate, ConsolidationRunJobInsert,
	},
	models::{ConsolidationProposal, ConsolidationRun},
};

impl ElfService {
	/// Creates a fixture-backed consolidation run and optional proposals.
	pub async fn consolidation_run_create(
		&self,
		req: ConsolidationRunCreateRequest,
	) -> Result<ConsolidationRunCreateResponse> {
		validation::validate_context(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
		)?;
		validation::validate_job_kind(req.job_kind.as_str())?;
		consolidation::validate_source_refs(&req.input_refs).map_err(validation_error)?;
		validation::validate_object("source_snapshot", &req.source_snapshot)?;

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
			input_refs: validation::to_value(&req.input_refs)?,
			source_snapshot: req.source_snapshot,
			lineage: validation::to_value(&req.lineage)?,
			error: types::empty_object(),
			created_at: now,
			updated_at: now,
			completed_at: validation::terminal_time(run_state, now),
		};
		let payload_value = validation::to_value(&payload)?;
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
		let limit = validation::bounded_limit(req.limit);
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
		let limit = validation::bounded_limit(req.limit);
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
		validation::validate_context(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.reviewer_agent_id.as_str(),
		)?;

		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let existing = elf_storage::consolidation::lock_consolidation_proposal(
			&mut *tx,
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
		let steps = validation::review_steps(current, req.review_action)?;
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
				promotion::create_promoted_memory_note(
					tx,
					&proposal,
					reviewer_agent_id,
					review_comment,
					&self.cfg,
					now,
				)
				.await?,
			"update_derived_note" =>
				promotion::update_promoted_memory_note(
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
		let target_ref = promotion::promoted_memory_target_ref(note_id, now);

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
