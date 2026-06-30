mod dreaming_readback_artifacts;

use crate::{
	AGENT_ID, DreamingReadbackMaterializationEvidence, DreamingReadbackOutput, ElfService,
	ListRequest, LoadedJob, OffsetDateTime, Result, Rfc3339, SCOPE, SearchResponse,
	SelectedEvidenceText, SuiteMaterializationSelection, SuiteMaterializationSelectionInput,
	TENANT_ID, Uuid, Value, eyre,
};

pub(super) fn search_response_evidence_ids(response: &SearchResponse) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	for item in &response.items {
		if let Some(evidence_id) = item.source_ref.get("evidence_id").and_then(Value::as_str) {
			crate::push_unique(&mut evidence_ids, evidence_id.to_string());
		}
	}

	evidence_ids
}

pub(super) fn suite_materialization_selection(
	input: SuiteMaterializationSelectionInput<'_>,
) -> SuiteMaterializationSelection {
	let suite_claims_materialized = input.capture_failure.is_none()
		&& ((input.loaded.job.suite == "knowledge_compilation" && input.knowledge.is_some())
			|| (input.loaded.job.suite == "consolidation" && input.consolidation.is_some())
			|| input.dreaming_readback.is_some());
	let selected = if let Some(output) = &input.dreaming_readback {
		SelectedEvidenceText {
			content: output.content.clone(),
			evidence_ids: output.evidence_ids.clone(),
		}
	} else if suite_claims_materialized {
		crate::expected_claim_text(
			input.loaded,
			crate::live_required_evidence_ids(input.loaded, input.ingested).as_slice(),
		)
	} else {
		input.selected
	};
	let trace_stages = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.trace_stages.clone())
		.or(input.trace_stages);
	let memory_summaries = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.memory_summaries.clone())
		.unwrap_or_default();
	let proactive_briefs = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.proactive_briefs.clone())
		.unwrap_or_default();
	let scheduled_tasks = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.scheduled_tasks.clone())
		.unwrap_or_default();
	let dreaming_readback =
		input.dreaming_readback.as_ref().map(|output| output.materialization.clone());

	SuiteMaterializationSelection {
		selected,
		trace_stages,
		dreaming_readback,
		memory_summaries,
		proactive_briefs,
		scheduled_tasks,
	}
}

pub(super) async fn materialize_elf_dreaming_readback(
	service: &ElfService,
	loaded: &LoadedJob,
	project_id: &str,
	trace_id: Uuid,
	adapter_id: &str,
) -> Result<Option<DreamingReadbackOutput>> {
	if !crate::is_elf_dreaming_readback_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return Ok(None);
	}

	let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
	let service_evidence_ids = service_readback_evidence_ids(service, project_id).await?;
	let mut artifacts = dreaming_readback_artifacts::dreaming_readback_template_artifacts(loaded)?;

	for artifact in &mut artifacts {
		dreaming_readback_artifacts::stamp_dreaming_readback_artifact(
			artifact,
			loaded,
			project_id,
			trace_id,
			generated_at.as_str(),
		);
	}

	let mut artifact_source_refs = Vec::new();

	for artifact in &artifacts {
		dreaming_readback_artifacts::collect_dreaming_artifact_source_refs(
			artifact,
			&mut artifact_source_refs,
		);
	}

	artifact_source_refs.sort();
	artifact_source_refs.dedup();

	let missing_source_refs = artifact_source_refs
		.iter()
		.filter(|source_ref| !service_evidence_ids.contains(*source_ref))
		.cloned()
		.collect::<Vec<_>>();
	let returned_source_refs = artifact_source_refs
		.iter()
		.filter(|source_ref| service_evidence_ids.contains(*source_ref))
		.cloned()
		.collect::<Vec<_>>();
	let scoring_evidence_ids = dreaming_readback_artifacts::dreaming_readback_scoring_evidence_ids(
		loaded,
		&service_evidence_ids,
	);
	let artifact_kind = match loaded.job.suite.as_str() {
		"memory_summary" => "elf.memory_summary/v1",
		"proactive_brief" => "elf.proactive_project_brief/v1",
		"scheduled_memory" => "elf.scheduled_memory_task/v1",
		_ => "elf.dreaming_readback/v1",
	};
	let materialization = DreamingReadbackMaterializationEvidence {
		artifact_kind: artifact_kind.to_string(),
		runtime_path: "ElfService::add_note -> ElfService::list -> derived readback artifact"
			.to_string(),
		service_list_count: service_evidence_ids.len(),
		trace_id: Some(trace_id),
		generated_artifact_count: artifacts.len(),
		selected_source_refs: returned_source_refs.clone(),
		missing_source_refs,
		source_mutation_count: 0,
		no_source_mutation_checked: true,
	};
	let trace_stages =
		dreaming_readback_artifacts::dreaming_readback_trace_stages(loaded, &materialization);
	let content = dreaming_readback_artifacts::dreaming_readback_content(
		loaded.job.suite.as_str(),
		&artifacts,
	);
	let (memory_summaries, proactive_briefs, scheduled_tasks) = match loaded.job.suite.as_str() {
		"memory_summary" => (artifacts, Vec::new(), Vec::new()),
		"proactive_brief" => (Vec::new(), artifacts, Vec::new()),
		"scheduled_memory" => (Vec::new(), Vec::new(), artifacts),
		_ => (Vec::new(), Vec::new(), Vec::new()),
	};

	Ok(Some(DreamingReadbackOutput {
		content,
		evidence_ids: scoring_evidence_ids,
		memory_summaries,
		proactive_briefs,
		scheduled_tasks,
		materialization,
		trace_stages,
	}))
}

async fn service_readback_evidence_ids(
	service: &ElfService,
	project_id: &str,
) -> Result<Vec<String>> {
	let response = service
		.list(ListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			agent_id: Some(AGENT_ID.to_string()),
			scope: Some(SCOPE.to_string()),
			status: Some("active".to_string()),
			r#type: None,
		})
		.await
		.map_err(|err| eyre::eyre!("ELF service-native readback list failed: {err}"))?;
	let mut evidence_ids = Vec::new();

	for item in response.items {
		if let Some(evidence_id) = item.source_ref.get("evidence_id").and_then(Value::as_str) {
			crate::push_unique(&mut evidence_ids, evidence_id.to_string());
		}
	}

	Ok(evidence_ids)
}
