use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	consolidation::{
		types::{
			self, ConsolidationProposalInput, ConsolidationRunCreateRequest,
			ConsolidationRunCreateResponse, ConsolidationRunGetRequest, ConsolidationRunResponse,
			ConsolidationRunsListRequest, ConsolidationRunsListResponse,
		},
		validation::{self, validation_error},
	},
};
use elf_domain::consolidation::{
	self, CONSOLIDATION_CONTRACT_SCHEMA_V1, ConsolidationJobPayload, ConsolidationRunState,
};
use elf_storage::{consolidation::ConsolidationRunJobInsert, models::ConsolidationRun};

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
}
