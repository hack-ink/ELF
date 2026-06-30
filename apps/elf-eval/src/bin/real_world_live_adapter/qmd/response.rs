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
		let entry_text = serde_json::to_string(entry)?;

		for item in corpus {
			if entry_text.contains(format!("{}.md", crate::slug(&item.evidence_id)).as_str())
				|| entry_text.contains(item.evidence_id.as_str())
			{
				crate::push_unique(&mut evidence_ids, item.evidence_id.clone());
			}
		}
	}

	Ok((entries, evidence_ids))
}

pub(super) fn qmd_materialized_job(
	loaded: &LoadedJob,
	adapter_id: &str,
	selected: SelectedEvidenceText,
	latency_ms: f64,
	returned_count: usize,
	operator_debug: Option<Value>,
	operator_debug_evidence: Option<OperatorDebugMaterializationEvidence>,
) -> MaterializedJob {
	crate::materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content: selected.content,
			evidence_ids: selected.evidence_ids,
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
