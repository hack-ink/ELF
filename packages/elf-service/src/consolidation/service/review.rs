use sqlx::{Postgres, Transaction};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	consolidation::{
		promotion::{self},
		types::{
			ConsolidationProposalResponse, ConsolidationProposalReviewEventResponse,
			ConsolidationProposalReviewRequest,
		},
		validation::{self, validation_error},
	},
};
use elf_domain::consolidation::{ConsolidationReviewAction, ConsolidationReviewState};
use elf_storage::{
	consolidation::{
		self, ConsolidationProposalReviewEventInsert, ConsolidationProposalReviewUpdate,
		ConsolidationProposalTargetRefUpdate,
	},
	models::ConsolidationProposal,
};

impl ElfService {
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
		let existing = consolidation::lock_consolidation_proposal(
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

			consolidation::insert_consolidation_proposal_review_event(
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

			updated = consolidation::update_consolidation_proposal_review(
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

		let review_events = proposal_review_events(
			self,
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

		consolidation::update_consolidation_proposal_target_ref(
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
}

pub(super) async fn proposal_review_events(
	service: &ElfService,
	tenant_id: &str,
	project_id: &str,
	proposal_id: Uuid,
) -> Result<Vec<ConsolidationProposalReviewEventResponse>> {
	let events = consolidation::list_consolidation_proposal_review_events(
		&service.db.pool,
		tenant_id,
		project_id,
		proposal_id,
	)
	.await?;

	Ok(events.into_iter().map(ConsolidationProposalReviewEventResponse::from).collect())
}
