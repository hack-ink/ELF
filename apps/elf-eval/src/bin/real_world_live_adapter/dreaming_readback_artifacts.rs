use crate::{
	AGENT_ID, BTreeSet, DreamingReadbackMaterializationEvidence, LoadedJob, Result, TENANT_ID,
	TraceStageOutput, Uuid, Value, eyre, serde_json,
};

pub(super) fn dreaming_readback_template_artifacts(loaded: &LoadedJob) -> Result<Vec<Value>> {
	let pointer = match loaded.job.suite.as_str() {
		"memory_summary" => "/corpus/adapter_response/answer/memory_summaries",
		"proactive_brief" => "/corpus/adapter_response/answer/proactive_briefs",
		"scheduled_memory" => "/corpus/adapter_response/answer/scheduled_tasks",
		_ => return Ok(Vec::new()),
	};
	let artifacts =
		loaded.value.pointer(pointer).and_then(Value::as_array).cloned().ok_or_else(|| {
			eyre::eyre!(
				"{} missing service-native readback template at {pointer}.",
				loaded.job.job_id
			)
		})?;

	if artifacts.is_empty() {
		return Err(eyre::eyre!(
			"{} has no service-native readback template artifacts.",
			loaded.job.job_id
		));
	}

	Ok(artifacts)
}

pub(super) fn dreaming_readback_scoring_evidence_ids(
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
			crate::push_unique(&mut evidence_ids, evidence.evidence_id.clone());
		}
	}

	if evidence_ids.is_empty() {
		for evidence_id in service_evidence_ids {
			if !trap_ids.contains(evidence_id.as_str()) {
				crate::push_unique(&mut evidence_ids, evidence_id.clone());
			}
		}
	}

	evidence_ids
}

pub(super) fn stamp_dreaming_readback_artifact(
	artifact: &mut Value,
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

pub(super) fn collect_dreaming_artifact_source_refs(value: &Value, refs: &mut Vec<String>) {
	match value {
		Value::Array(items) =>
			for item in items {
				collect_dreaming_artifact_source_refs(item, refs);
			},
		Value::Object(map) =>
			for (key, value) in map {
				if matches!(key.as_str(), "source_refs" | "evidence_refs" | "evidence_ids")
					&& let Some(items) = value.as_array()
				{
					for item in items {
						if let Some(source_ref) = item.as_str() {
							crate::push_unique(refs, source_ref.to_string());
						}
					}
				}
				if key == "evidence_id"
					&& let Some(source_ref) = value.as_str()
				{
					crate::push_unique(refs, source_ref.to_string());
				}

				collect_dreaming_artifact_source_refs(value, refs);
			},
		_ => {},
	}
}

pub(super) fn dreaming_readback_content(suite: &str, artifacts: &[Value]) -> String {
	let mut parts = Vec::new();

	for artifact in artifacts {
		match suite {
			"memory_summary" => {
				for entry in artifact.get("entries").and_then(Value::as_array).into_iter().flatten()
				{
					if let Some(text) = entry.get("text").and_then(Value::as_str) {
						parts.push(text.to_string());
					}
				}
			},
			"proactive_brief" => {
				for suggestion in
					artifact.get("suggestions").and_then(Value::as_array).into_iter().flatten()
				{
					if let Some(title) = suggestion.get("title").and_then(Value::as_str) {
						parts.push(title.to_string());
					}
					if let Some(body) = suggestion.get("body").and_then(Value::as_str) {
						parts.push(body.to_string());
					}
				}
			},
			"scheduled_memory" => {
				for output in
					artifact.get("outputs").and_then(Value::as_array).into_iter().flatten()
				{
					if let Some(text) = output.get("text").and_then(Value::as_str) {
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

pub(super) fn dreaming_readback_trace_stages(
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

fn negative_trap_evidence_ids(loaded: &LoadedJob) -> BTreeSet<&str> {
	loaded
		.value
		.get("negative_traps")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter(|trap| trap.get("failure_if_used").and_then(Value::as_bool).unwrap_or(false))
		.flat_map(|trap| {
			trap.get("evidence_ids")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(Value::as_str)
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use crate::{DreamingReadbackMaterializationEvidence, LoadedJob, Uuid, Value, serde_json};

	fn loaded_job(suite: &str, adapter_answer: Value) -> LoadedJob {
		let value = serde_json::json!({
			"schema": "elf.real_world_job/v1",
			"job_id": format!("{suite}-job"),
			"suite": suite,
			"title": "Service native readback fixture",
			"corpus": {
				"items": [],
				"adapter_response": {
					"answer": adapter_answer
				}
			},
			"prompt": { "content": "read back service artifacts" },
			"expected_answer": { "must_include": [], "evidence_links": {} },
			"required_evidence": [
				{ "evidence_id": "evidence-a" },
				{ "evidence_id": "evidence-b" },
				{ "evidence_id": "trap-evidence" }
			],
			"memory_evolution": null,
			"negative_traps": [
				{
					"failure_if_used": true,
					"evidence_ids": ["trap-evidence"]
				}
			]
		});
		let job = serde_json::from_value(value.clone()).expect("fixture job parses");

		LoadedJob { path: PathBuf::from(format!("{suite}.json")), value, job }
	}

	#[test]
	fn template_artifacts_select_suite_specific_adapter_answer() {
		let memory = loaded_job(
			"memory_summary",
			serde_json::json!({
				"memory_summaries": [{ "entries": [{ "text": "alpha memory" }] }]
			}),
		);
		let proactive = loaded_job(
			"proactive_brief",
			serde_json::json!({
				"proactive_briefs": [{ "suggestions": [{ "title": "next", "body": "step" }] }]
			}),
		);
		let scheduled = loaded_job(
			"scheduled_memory",
			serde_json::json!({
				"scheduled_tasks": [{ "outputs": [{ "text": "scheduled note" }] }]
			}),
		);
		let unsupported = loaded_job("other_suite", serde_json::json!({}));

		assert_eq!(super::dreaming_readback_template_artifacts(&memory).unwrap().len(), 1);
		assert_eq!(super::dreaming_readback_template_artifacts(&proactive).unwrap().len(), 1);
		assert_eq!(super::dreaming_readback_template_artifacts(&scheduled).unwrap().len(), 1);
		assert!(super::dreaming_readback_template_artifacts(&unsupported).unwrap().is_empty());
	}

	#[test]
	fn scoring_evidence_prefers_required_matches_and_filters_negative_traps() {
		let loaded = loaded_job(
			"memory_summary",
			serde_json::json!({
				"memory_summaries": [{ "entries": [{ "text": "alpha memory" }] }]
			}),
		);

		assert_eq!(
			super::dreaming_readback_scoring_evidence_ids(
				&loaded,
				&["evidence-b".to_string(), "trap-evidence".to_string()]
			),
			vec!["evidence-b"]
		);
		assert_eq!(
			super::dreaming_readback_scoring_evidence_ids(
				&loaded,
				&["fallback-evidence".to_string(), "trap-evidence".to_string()]
			),
			vec!["fallback-evidence"]
		);
	}

	#[test]
	fn stamp_and_collect_source_refs_preserve_runtime_metadata_and_dedup_nested_refs() {
		let loaded = loaded_job(
			"scheduled_memory",
			serde_json::json!({
				"scheduled_tasks": [{ "outputs": [{ "text": "scheduled note" }] }]
			}),
		);
		let trace_id = Uuid::nil();
		let mut artifact = serde_json::json!({
			"evidence_id": "evidence-a",
			"source_refs": ["evidence-b", "evidence-a"],
			"nested": {
				"evidence_refs": ["evidence-c"],
				"items": [{ "evidence_ids": ["evidence-c", "evidence-d"] }]
			}
		});

		super::stamp_dreaming_readback_artifact(
			&mut artifact,
			&loaded,
			"project-1",
			trace_id,
			"2026-06-30T00:00:00Z",
		);

		assert_eq!(artifact["project_id"], "project-1");
		assert_eq!(artifact["service_readback"]["runtime_path"], "ElfService::list");
		assert_eq!(artifact["execution_trace"]["status"], "completed");
		assert_eq!(artifact["source_mutations"], serde_json::json!([]));

		let mut refs = Vec::new();

		super::collect_dreaming_artifact_source_refs(&artifact, &mut refs);

		refs.sort();

		assert_eq!(refs, vec!["evidence-a", "evidence-b", "evidence-c", "evidence-d"]);
	}

	#[test]
	fn content_and_trace_stages_encode_artifact_text_and_source_mutation_guard() {
		let loaded = loaded_job(
			"proactive_brief",
			serde_json::json!({
				"proactive_briefs": [{
					"suggestions": [{ "title": "Review", "body": "Follow the trace" }]
				}]
			}),
		);
		let artifact_values = super::dreaming_readback_template_artifacts(&loaded).unwrap();
		let evidence = DreamingReadbackMaterializationEvidence {
			selected_source_refs: vec!["evidence-a".to_string()],
			missing_source_refs: vec!["missing-evidence".to_string()],
			..DreamingReadbackMaterializationEvidence::default()
		};

		assert_eq!(
			super::dreaming_readback_content("proactive_brief", &artifact_values),
			"Review Follow the trace"
		);

		let stages = super::dreaming_readback_trace_stages(&loaded, &evidence);

		assert_eq!(stages.len(), 2);
		assert_eq!(stages[0].stage_name, "dreaming_readback.service_list");
		assert_eq!(stages[0].kept_evidence, vec!["evidence-a"]);
		assert_eq!(stages[0].dropped_evidence, vec!["missing-evidence"]);
		assert_eq!(stages[1].stage_name, "dreaming_readback.source_mutation_guard");
	}
}
