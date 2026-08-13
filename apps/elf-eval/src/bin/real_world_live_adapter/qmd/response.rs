use crate::{
	CorpusText, LoadedJob, MaterializedJob, MaterializedJobInput,
	OperatorDebugMaterializationEvidence, Result, SelectedEvidenceText, Value, eyre, serde_json,
};

pub(super) fn qmd_query_entries(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	stdout: &str,
) -> Result<(Vec<Value>, Vec<String>)> {
	let results = serde_json::from_str::<Value>(stdout).map_err(|err| {
		eyre::eyre!("qmd query did not return JSON for {}: {err}", loaded.job.job_id)
	})?;
	let entries = results.as_array().cloned().unwrap_or_default();
	let mut evidence_ids = Vec::new();

	for entry in &entries {
		if let Some(evidence_id) = qmd_entry_evidence_id(entry, corpus)? {
			crate::push_unique(&mut evidence_ids, evidence_id);
		}
	}

	Ok((entries, evidence_ids))
}

fn qmd_entry_evidence_id(entry: &Value, corpus: &[CorpusText]) -> Result<Option<String>> {
	let entry_text = serde_json::to_string(entry)?;
	Ok(corpus
		.iter()
		.find(|item| {
			entry_text.contains(format!("{}.md", crate::slug(&item.evidence_id)).as_str())
				|| entry_text.contains(item.evidence_id.as_str())
		})
		.map(|item| item.evidence_id.clone()))
}

pub(super) fn qmd_native_contexts(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	entries: &[Value],
) -> Result<Vec<Value>> {
	entries
		.iter()
		.map(|entry| {
			let text = entry
				.get("snippet")
				.or_else(|| entry.get("body"))
				.and_then(Value::as_str)
				.ok_or_else(|| {
				eyre::eyre!("qmd query returned no native text for {}.", loaded.job.job_id)
			})?;
			let evidence_id = qmd_entry_evidence_id(entry, corpus)?;
			Ok(serde_json::json!({"evidence_id": evidence_id, "text": text}))
		})
		.collect()
}

pub(super) fn qmd_materialized_job(
	loaded: &LoadedJob,
	adapter_id: &str,
	selected: SelectedEvidenceText,
	contexts: Vec<Value>,
	latency_ms: f64,
	returned_count: usize,
	operator_debug: Option<Value>,
	operator_debug_evidence: Option<OperatorDebugMaterializationEvidence>,
) -> MaterializedJob {
	let native_content = contexts
		.iter()
		.filter_map(|context| context.get("text").and_then(Value::as_str))
		.collect::<Vec<_>>()
		.join("\n");
	let content = if loaded.job.operations.is_empty() { selected.content } else { native_content };
	crate::materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content,
			evidence_ids: selected.evidence_ids,
			contexts: Some(contexts),
			pages: Vec::new(),
			latency_ms,
			indexing_latency_ms: None,
			returned_count,
			trace_id: None,
			failure: None,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: None,
			capture_failure: None,
			consolidation_response: None,
			consolidation: None,
			knowledge: None,
			temporal_reconciliation: None,
			dreaming_readback: None,
			memory_summaries: Vec::new(),
			proactive_briefs: Vec::new(),
			scheduled_tasks: Vec::new(),
			trace_stages: None,
		},
	)
}
