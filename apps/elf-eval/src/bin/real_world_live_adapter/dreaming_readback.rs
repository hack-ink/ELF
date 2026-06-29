use super::*;

fn dreaming_readback_template_artifacts(
	loaded: &LoadedJob,
) -> color_eyre::Result<Vec<serde_json::Value>> {
	let pointer = match loaded.job.suite.as_str() {
		"memory_summary" => "/corpus/adapter_response/answer/memory_summaries",
		"proactive_brief" => "/corpus/adapter_response/answer/proactive_briefs",
		"scheduled_memory" => "/corpus/adapter_response/answer/scheduled_tasks",
		_ => return Ok(Vec::new()),
	};
	let artifacts =
		loaded.value.pointer(pointer).and_then(serde_json::Value::as_array).cloned().ok_or_else(
			|| {
				eyre::eyre!(
					"{} missing service-native readback template at {pointer}.",
					loaded.job.job_id
				)
			},
		)?;

	if artifacts.is_empty() {
		return Err(eyre::eyre!(
			"{} has no service-native readback template artifacts.",
			loaded.job.job_id
		));
	}

	Ok(artifacts)
}

fn dreaming_readback_scoring_evidence_ids(
	loaded: &LoadedJob,
	service_evidence_ids: &[String],
) -> Vec<String> {
	let selected = service_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let trap_ids = negative_trap_evidence_ids(loaded);
	let mut evidence_ids = Vec::new();

	for evidence in &loaded.job.required_evidence {
		if selected.contains(evidence.evidence_id.as_str())
			&& !trap_ids.contains(evidence.evidence_id.as_str())
		{
			push_unique(&mut evidence_ids, evidence.evidence_id.clone());
		}
	}

	if evidence_ids.is_empty() {
		for evidence_id in service_evidence_ids {
			if !trap_ids.contains(evidence_id.as_str()) {
				push_unique(&mut evidence_ids, evidence_id.clone());
			}
		}
	}

	evidence_ids
}

fn negative_trap_evidence_ids(loaded: &LoadedJob) -> BTreeSet<&str> {
	loaded
		.value
		.get("negative_traps")
		.and_then(serde_json::Value::as_array)
		.into_iter()
		.flatten()
		.filter(|trap| {
			trap.get("failure_if_used").and_then(serde_json::Value::as_bool).unwrap_or(false)
		})
		.flat_map(|trap| {
			trap.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(serde_json::Value::as_str)
		})
		.collect()
}

fn stamp_dreaming_readback_artifact(
	artifact: &mut serde_json::Value,
	loaded: &LoadedJob,
	project_id: &str,
	trace_id: Uuid,
	generated_at: &str,
) {
	artifact["generated_at"] = serde_json::json!(generated_at);
	artifact["tenant_id"] = serde_json::json!(TENANT_ID);
	artifact["project_id"] = serde_json::json!(project_id);
	artifact["agent_id"] = serde_json::json!(AGENT_ID);
	artifact["read_profile"] = serde_json::json!("private_only");
	artifact["service_readback"] = serde_json::json!({
		"schema": "elf.service_native_dreaming_readback/v1",
		"job_id": loaded.job.job_id,
		"suite": loaded.job.suite,
		"runtime_path": "ElfService::list",
		"search_trace_id": trace_id,
		"source_mutation_count": 0
	});

	if loaded.job.suite == "scheduled_memory" {
		let trace = artifact
			.as_object_mut()
			.map(|object| object.entry("execution_trace").or_insert_with(|| serde_json::json!({})));

		if let Some(trace) = trace {
			trace["trace_id"] = serde_json::json!(format!("service-native-{trace_id}"));
			trace["trigger_kind"] = serde_json::json!("service_native_readback");
			trace["status"] = serde_json::json!("completed");
		}

		artifact["source_mutations"] = serde_json::json!([]);
	}
}

fn collect_dreaming_artifact_source_refs(value: &serde_json::Value, refs: &mut Vec<String>) {
	match value {
		serde_json::Value::Array(items) =>
			for item in items {
				collect_dreaming_artifact_source_refs(item, refs);
			},
		serde_json::Value::Object(map) =>
			for (key, value) in map {
				if matches!(key.as_str(), "source_refs" | "evidence_refs" | "evidence_ids")
					&& let Some(items) = value.as_array()
				{
					for item in items {
						if let Some(source_ref) = item.as_str() {
							push_unique(refs, source_ref.to_string());
						}
					}
				}
				if key == "evidence_id"
					&& let Some(source_ref) = value.as_str()
				{
					push_unique(refs, source_ref.to_string());
				}

				collect_dreaming_artifact_source_refs(value, refs);
			},
		_ => {},
	}
}

fn dreaming_readback_content(suite: &str, artifacts: &[serde_json::Value]) -> String {
	let mut parts = Vec::new();

	for artifact in artifacts {
		match suite {
			"memory_summary" => {
				for entry in artifact
					.get("entries")
					.and_then(serde_json::Value::as_array)
					.into_iter()
					.flatten()
				{
					if let Some(text) = entry.get("text").and_then(serde_json::Value::as_str) {
						parts.push(text.to_string());
					}
				}
			},
			"proactive_brief" => {
				for suggestion in artifact
					.get("suggestions")
					.and_then(serde_json::Value::as_array)
					.into_iter()
					.flatten()
				{
					if let Some(title) = suggestion.get("title").and_then(serde_json::Value::as_str)
					{
						parts.push(title.to_string());
					}
					if let Some(body) = suggestion.get("body").and_then(serde_json::Value::as_str) {
						parts.push(body.to_string());
					}
				}
			},
			"scheduled_memory" => {
				for output in artifact
					.get("outputs")
					.and_then(serde_json::Value::as_array)
					.into_iter()
					.flatten()
				{
					if let Some(text) = output.get("text").and_then(serde_json::Value::as_str) {
						parts.push(text.to_string());
					}
				}
			},
			_ => {},
		}
	}

	if parts.is_empty() {
		"Service-native Dreaming readback produced no artifact text.".to_string()
	} else {
		parts.join(" ")
	}
}

fn dreaming_readback_trace_stages(
	loaded: &LoadedJob,
	evidence: &DreamingReadbackMaterializationEvidence,
) -> Vec<TraceStageOutput> {
	vec![
		TraceStageOutput {
			stage_name: "dreaming_readback.service_list".to_string(),
			kept_evidence: evidence.selected_source_refs.clone(),
			dropped_evidence: evidence.missing_source_refs.clone(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: format!(
				"Read {} source refs from ElfService::list for {}.",
				evidence.selected_source_refs.len(),
				loaded.job.suite
			),
		},
		TraceStageOutput {
			stage_name: "dreaming_readback.source_mutation_guard".to_string(),
			kept_evidence: evidence.selected_source_refs.clone(),
			dropped_evidence: Vec::new(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Generated readback artifacts without mutating source notes.".to_string(),
		},
	]
}

pub(super) fn search_response_evidence_ids(response: &SearchResponse) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	for item in &response.items {
		if let Some(evidence_id) =
			item.source_ref.get("evidence_id").and_then(serde_json::Value::as_str)
		{
			push_unique(&mut evidence_ids, evidence_id.to_string());
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
		expected_claim_text(
			input.loaded,
			live_required_evidence_ids(input.loaded, input.ingested).as_slice(),
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
) -> color_eyre::Result<Option<DreamingReadbackOutput>> {
	if !is_elf_dreaming_readback_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return Ok(None);
	}

	let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
	let service_evidence_ids = service_readback_evidence_ids(service, project_id).await?;
	let mut artifacts = dreaming_readback_template_artifacts(loaded)?;

	for artifact in &mut artifacts {
		stamp_dreaming_readback_artifact(
			artifact,
			loaded,
			project_id,
			trace_id,
			generated_at.as_str(),
		);
	}

	let mut artifact_source_refs = Vec::new();

	for artifact in &artifacts {
		collect_dreaming_artifact_source_refs(artifact, &mut artifact_source_refs);
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
	let scoring_evidence_ids =
		dreaming_readback_scoring_evidence_ids(loaded, &service_evidence_ids);
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
	let trace_stages = dreaming_readback_trace_stages(loaded, &materialization);
	let content = dreaming_readback_content(loaded.job.suite.as_str(), &artifacts);
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
) -> color_eyre::Result<Vec<String>> {
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
		if let Some(evidence_id) =
			item.source_ref.get("evidence_id").and_then(serde_json::Value::as_str)
		{
			push_unique(&mut evidence_ids, evidence_id.to_string());
		}
	}

	Ok(evidence_ids)
}
