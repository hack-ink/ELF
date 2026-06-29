use crate::{
	AGENT_ID, AddNoteInput, AddNoteRequest, BaselineRuntime, ConsolidationLineage,
	ConsolidationMaterializationEvidence, ConsolidationProposalResponse,
	ConsolidationProposalReviewRequest, ConsolidationProposalsListRequest,
	ConsolidationRunCreateRequest, ElfService, IngestedCorpus, KnowledgeMaterializationEvidence,
	KnowledgePageKind, KnowledgePageLintRequest, KnowledgePageRebuildRequest,
	KnowledgePageSearchRequest, LiveConsolidationFixture, LoadedJob, Result, SCOPE, TENANT_ID,
	Uuid, Value, eyre, serde_json,
};

pub(super) async fn materialize_elf_consolidation(
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

pub(super) async fn materialize_elf_knowledge(
	service: &ElfService,
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	adapter_id: &str,
) -> Result<(Vec<Value>, Option<KnowledgeMaterializationEvidence>, Option<String>)> {
	if loaded.job.suite != "knowledge_compilation" {
		return Ok((Vec::new(), None, None));
	}

	let project_id = crate::project_id_for_job(&loaded.job.job_id);
	let note_ids = crate::live_note_ids(ingested);

	if note_ids.is_empty() {
		return Err(eyre::eyre!(
			"{} has no live note sources for knowledge rebuild.",
			loaded.job.job_id
		));
	}

	let page_key = crate::slug(&loaded.job.job_id);
	let request = KnowledgePageRebuildRequest {
		tenant_id: TENANT_ID.to_string(),
		project_id: project_id.clone(),
		agent_id: AGENT_ID.to_string(),
		page_kind: KnowledgePageKind::Project,
		page_key,
		title: Some(loaded.job.title.clone()),
		doc_ids: Vec::new(),
		doc_chunk_ids: Vec::new(),
		note_ids: note_ids.clone(),
		event_ids: Vec::new(),
		relation_ids: Vec::new(),
		proposal_ids: Vec::new(),
		provider_metadata: serde_json::json!({
			"adapter_id": adapter_id,
			"job_id": loaded.job.job_id,
			"llm_derived": false,
			"runtime_path": "ElfService::knowledge_page_rebuild"
		}),
	};
	let first = service.knowledge_page_rebuild(request.clone()).await.map_err(|err| {
		eyre::eyre!("ELF knowledge_page_rebuild failed for {}: {err}", loaded.job.job_id)
	})?;
	let second = service.knowledge_page_rebuild(request).await.map_err(|err| {
		eyre::eyre!("ELF second knowledge_page_rebuild failed for {}: {err}", loaded.job.job_id)
	})?;

	update_stale_trap_sources(service, loaded, adapter_id, project_id.as_str()).await?;

	let lint = service
		.knowledge_page_lint(KnowledgePageLintRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.clone(),
			page_id: second.page.page.page_id,
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF knowledge_page_lint failed for {}: {err}", loaded.job.job_id)
		})?;
	let search = service
		.knowledge_pages_search(KnowledgePageSearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id,
			agent_id: AGENT_ID.to_string(),
			read_profile: "private_only".to_string(),
			query: "source notes".to_string(),
			page_kind: Some(KnowledgePageKind::Project),
			limit: Some(10),
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF knowledge_pages_search failed for {}: {err}", loaded.job.job_id)
		})?;
	let page = crate::knowledge_page_artifact(loaded, ingested, &first.page, &second.page, &lint)?;
	let evidence =
		crate::knowledge_materialization_evidence(&second.page, &lint, search.items.len());

	Ok((vec![page], Some(evidence), None))
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

async fn update_stale_trap_sources(
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
	project_id: &str,
) -> Result<()> {
	for evidence_id in crate::stale_trap_evidence_ids(loaded) {
		service
			.add_note(AddNoteRequest {
				tenant_id: TENANT_ID.to_string(),
				project_id: project_id.to_string(),
				agent_id: AGENT_ID.to_string(),
				scope: SCOPE.to_string(),
				notes: vec![AddNoteInput {
					r#type: "fact".to_string(),
					key: Some(evidence_id.clone()),
					text: format!(
						"Current lint probe: evidence {evidence_id} changed after the knowledge page rebuild and should mark the derived page source snapshot stale."
					),
					structured: None,
					importance: 0.9,
					confidence: 0.95,
					ttl_days: None,
					source_ref: serde_json::json!({
						"schema": "real_world_live_adapter/v1",
						"adapter": adapter_id,
						"job_id": loaded.job.job_id,
						"evidence_id": evidence_id,
						"lint_probe": "stale_source_ref"
					}),
					write_policy: None,
				}],
			})
			.await
			.map_err(|err| {
				eyre::eyre!(
					"ELF add_note stale-source update failed for {}: {err}",
					loaded.job.job_id
				)
			})?;
	}

	Ok(())
}
