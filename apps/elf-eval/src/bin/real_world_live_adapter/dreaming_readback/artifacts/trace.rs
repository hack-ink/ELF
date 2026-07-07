use crate::{DreamingReadbackMaterializationEvidence, LoadedJob, TraceStageOutput};

pub(in crate::dreaming_readback) fn dreaming_readback_trace_stages(
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
