use crate::knowledge::{
	BTreeSet, ConsolidationInputRef, ConsolidationRunCreateResponse, KnowledgeDeltaMemoryCandidate,
	KnowledgePageChangedSource, KnowledgePageProposalRunSummary, Value,
};

pub(in crate::knowledge) fn candidate_run_input_refs(
	candidates: &[KnowledgeDeltaMemoryCandidate],
) -> Vec<ConsolidationInputRef> {
	let mut seen = BTreeSet::new();
	let mut out = Vec::new();

	for source_ref in candidates.iter().flat_map(|candidate| &candidate.source_refs) {
		if seen.insert((source_ref.kind.as_str().to_string(), source_ref.id)) {
			out.push(source_ref.clone());
		}
	}

	out
}

pub(in crate::knowledge) fn knowledge_delta_source_snapshot(
	changed_sources: &[KnowledgePageChangedSource],
	candidates: &[KnowledgeDeltaMemoryCandidate],
) -> Value {
	serde_json::json!({
		"schema": "elf.knowledge_delta.run_source_snapshot/v1",
		"changed_sources": changed_sources,
		"candidate_count": candidates.len(),
		"candidate_reasons": candidates
			.iter()
			.map(|candidate| candidate.reason.clone())
			.collect::<Vec<_>>(),
		"source_mutation_allowed": false,
	})
}

pub(in crate::knowledge) fn proposal_run_summary(
	created: ConsolidationRunCreateResponse,
	proposal_count: usize,
) -> KnowledgePageProposalRunSummary {
	KnowledgePageProposalRunSummary {
		run_id: created.run.run_id,
		job_id: created.job_id,
		proposal_count,
		review_surface: "consolidation_proposals".to_string(),
	}
}
