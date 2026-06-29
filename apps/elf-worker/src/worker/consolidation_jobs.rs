use crate::worker::{
	self, CONSOLIDATION_CONTRACT_SCHEMA_V1, ConsolidationJobPayload, ConsolidationProposal,
	ConsolidationProposalContract, ConsolidationReviewState, ConsolidationRunJob,
	ConsolidationRunState, ConsolidationRunStateUpdate, ConsolidationValidationError, Db, Error,
	OffsetDateTime, Result, ToString, Uuid, Value, consolidation,
};

pub(super) fn proposal_row_from_contract(
	job: &ConsolidationRunJob,
	now: OffsetDateTime,
	proposal: ConsolidationProposalContract,
) -> Result<ConsolidationProposal> {
	proposal.validate().map_err(consolidation_validation_error)?;

	Ok(ConsolidationProposal {
		proposal_id: Uuid::new_v4(),
		run_id: job.run_id,
		tenant_id: job.tenant_id.clone(),
		project_id: job.project_id.clone(),
		agent_id: job.agent_id.clone(),
		contract_schema: CONSOLIDATION_CONTRACT_SCHEMA_V1.to_string(),
		proposal_kind: proposal.proposal_kind,
		apply_intent: proposal.apply_intent.as_str().to_string(),
		review_state: ConsolidationReviewState::Proposed.as_str().to_string(),
		source_refs: worker::encode_json(&proposal.source_refs, "consolidation source_refs")?,
		source_snapshot: proposal.source_snapshot,
		lineage: worker::encode_json(&proposal.lineage, "consolidation lineage")?,
		diff: worker::encode_json(&proposal.diff, "consolidation diff")?,
		confidence: proposal.confidence,
		unsupported_claim_flags: worker::encode_json(
			&proposal.unsupported_claim_flags,
			"consolidation unsupported_claim_flags",
		)?,
		contradiction_markers: worker::encode_json(
			&proposal.markers.contradictions,
			"consolidation contradiction_markers",
		)?,
		staleness_markers: worker::encode_json(
			&proposal.markers.staleness,
			"consolidation staleness_markers",
		)?,
		target_ref: proposal.target_ref,
		proposed_payload: proposal.proposed_payload,
		reviewer_agent_id: None,
		review_comment: None,
		reviewed_at: None,
		created_at: now,
		updated_at: now,
	})
}

pub(super) fn consolidation_validation_error(err: ConsolidationValidationError) -> Error {
	Error::Validation(err.to_string())
}

pub(super) async fn handle_consolidation_job(db: &Db, job: &ConsolidationRunJob) -> Result<()> {
	let payload: ConsolidationJobPayload = serde_json::from_value(job.payload.clone())?;

	payload.validate().map_err(consolidation_validation_error)?;

	let existing = consolidation::get_consolidation_run(
		&db.pool,
		job.tenant_id.as_str(),
		job.project_id.as_str(),
		job.run_id,
	)
	.await?
	.ok_or_else(|| Error::Validation("Consolidation run does not exist.".to_string()))?;
	let current_state =
		ConsolidationRunState::parse(existing.status.as_str()).ok_or_else(|| {
			Error::Validation("Stored consolidation run status is invalid.".to_string())
		})?;
	let now = OffsetDateTime::now_utc();
	let mut tx = db.pool.begin().await?;

	match current_state {
		ConsolidationRunState::Pending => {
			current_state
				.validate_transition(ConsolidationRunState::Running)
				.map_err(consolidation_validation_error)?;

			let empty_error = Value::Object(Default::default());

			consolidation::update_consolidation_run_state(
				&mut *tx,
				ConsolidationRunStateUpdate {
					tenant_id: job.tenant_id.as_str(),
					project_id: job.project_id.as_str(),
					run_id: job.run_id,
					status: ConsolidationRunState::Running.as_str(),
					error: &empty_error,
					now,
				},
			)
			.await?
			.ok_or_else(|| Error::Validation("Consolidation run disappeared.".to_string()))?;
		},
		ConsolidationRunState::Running => {},
		ConsolidationRunState::Completed
		| ConsolidationRunState::Failed
		| ConsolidationRunState::Cancelled => {
			consolidation::mark_consolidation_run_job_done(&mut *tx, job.job_id, now).await?;

			tx.commit().await?;

			return Ok(());
		},
	}

	for proposal in payload.proposals {
		let row = proposal_row_from_contract(job, now, proposal)?;

		consolidation::insert_consolidation_proposal(&mut *tx, &row).await?;
	}

	ConsolidationRunState::Running
		.validate_transition(ConsolidationRunState::Completed)
		.map_err(consolidation_validation_error)?;

	let empty_error = Value::Object(Default::default());

	consolidation::update_consolidation_run_state(
		&mut *tx,
		ConsolidationRunStateUpdate {
			tenant_id: job.tenant_id.as_str(),
			project_id: job.project_id.as_str(),
			run_id: job.run_id,
			status: ConsolidationRunState::Completed.as_str(),
			error: &empty_error,
			now,
		},
	)
	.await?
	.ok_or_else(|| Error::Validation("Consolidation run disappeared.".to_string()))?;
	consolidation::mark_consolidation_run_job_done(&mut *tx, job.job_id, now).await?;

	tx.commit().await?;

	Ok(())
}
