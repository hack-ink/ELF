mod content;
mod scoring;
mod source_refs;
mod stamp;
mod template;
mod trace;

pub(super) use self::{
	content::dreaming_readback_content, scoring::dreaming_readback_scoring_evidence_ids,
	source_refs::collect_dreaming_artifact_source_refs, stamp::stamp_dreaming_readback_artifact,
	template::dreaming_readback_template_artifacts, trace::dreaming_readback_trace_stages,
};

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
