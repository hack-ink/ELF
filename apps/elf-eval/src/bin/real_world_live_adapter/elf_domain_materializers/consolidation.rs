use crate::{
	AGENT_ID, BaselineRuntime, ConsolidationLineage, ConsolidationMaterializationEvidence,
	ConsolidationProposalResponse, ConsolidationProposalReviewRequest,
	ConsolidationProposalsListRequest, ConsolidationRunCreateRequest, ElfService, IngestedCorpus,
	LiveConsolidationFixture, LoadedJob, Result, TENANT_ID, Uuid, Value, eyre, serde_json,
};

pub(crate) async fn materialize_elf_consolidation(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	adapter_id: &str,
) -> Result<(Option<Value>, Option<ConsolidationMaterializationEvidence>, Option<String>)> {
	if loaded.job.suite != "consolidation" {
		return Ok((None, None, None));
	}

	let project_id = crate::project_id_for_job(&loaded.job.job_id);
	let fixture = crate::live_consolidation_fixture(loaded)?;
	let corpus = crate::corpus_texts(loaded)?;
	let prepared =
		crate::prepare_consolidation_run(loaded, adapter_id, ingested, &fixture, &corpus)?;
	let run = service
		.consolidation_run_create(ConsolidationRunCreateRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.clone(),
			agent_id: AGENT_ID.to_string(),
			job_kind: "fixture".to_string(),
			input_refs: prepared.input_refs.clone(),
			source_snapshot: serde_json::json!({
				"schema": "real_world_live_consolidation_run_snapshot/v1",
				"adapter_id": adapter_id,
				"job_id": loaded.job.job_id,
				"source_ref_count": prepared.input_refs.len()
			}),
			lineage: ConsolidationLineage {
				source_refs: prepared.input_refs.clone(),
				parent_run_id: None,
				parent_proposal_ids: Vec::new(),
			},
			proposals: prepared.proposals,
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF consolidation_run_create failed for {}: {err}", loaded.job.job_id)
		})?;

	crate::run_worker(runtime).await?;

	let reviewed = review_live_consolidation_proposals(
		service,
		loaded,
		project_id.as_str(),
		run.run.run_id,
		&fixture,
	)
	.await?;
	let consolidation_response = crate::live_consolidation_response(&fixture, &reviewed)?;
	let evidence = crate::consolidation_materialization_evidence(
		run.run.run_id,
		&fixture,
		&prepared.input_refs,
		&reviewed,
	);

	Ok((Some(consolidation_response), Some(evidence), None))
}

async fn review_live_consolidation_proposals(
	service: &ElfService,
	loaded: &LoadedJob,
	project_id: &str,
	run_id: Uuid,
	fixture: &LiveConsolidationFixture,
) -> Result<Vec<ConsolidationProposalResponse>> {
	let listed = service
		.consolidation_proposals_list(ConsolidationProposalsListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			run_id: Some(run_id),
			review_state: None,
			limit: Some(100),
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF consolidation proposal list failed for {}: {err}", loaded.job.job_id)
		})?;
	let mut reviewed = Vec::new();

	for (index, proposal) in listed.proposals.into_iter().enumerate() {
		let fixture_proposal = fixture.proposals.get(index).ok_or_else(|| {
			eyre::eyre!(
				"ELF consolidation materialized extra proposal {} for {}.",
				proposal.proposal_id,
				loaded.job.job_id
			)
		})?;
		let review_action =
			crate::consolidation_review_action(fixture_proposal.actual_review_action.as_str())?;

		reviewed.push(
			service
				.consolidation_proposal_review(ConsolidationProposalReviewRequest {
					tenant_id: TENANT_ID.to_string(),
					project_id: project_id.to_string(),
					reviewer_agent_id: AGENT_ID.to_string(),
					proposal_id: proposal.proposal_id,
					review_action,
					review_comment: Some(
						"Live adapter review transition for real-world benchmark evidence."
							.to_string(),
					),
				})
				.await
				.map_err(|err| {
					eyre::eyre!(
						"ELF consolidation proposal review failed for {}: {err}",
						loaded.job.job_id
					)
				})?,
		);
	}

	crate::validate_reviewed_consolidation_count(loaded, fixture, &reviewed)?;

	Ok(reviewed)
}
