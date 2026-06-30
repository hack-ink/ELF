mod content;
mod evidence;
mod ids;
mod trace;

use crate::evidence_selection::{
	self, BTreeSet, CorpusText, IngestedCorpus, LoadedJob, SelectedEvidenceText,
	TemporalReconciliationSelection,
};

pub(super) fn temporal_reconciliation_selection_impl(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	retrieved_evidence_ids: &[String],
	ingested: &IngestedCorpus,
) -> Option<TemporalReconciliationSelection> {
	let evolution = loaded.job.memory_evolution.as_ref()?;
	let relevant_ids = ids::temporal_reconciliation_relevant_ids(loaded, evolution);
	let retrieved_ids = retrieved_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let mut selected_ids = Vec::new();

	for evidence_id in &relevant_ids {
		if retrieved_ids.contains(evidence_id.as_str())
			&& ingested.note_ids_by_evidence.contains_key(evidence_id)
		{
			evidence_selection::push_unique(&mut selected_ids, evidence_id.clone());
		}
	}

	if selected_ids.is_empty() {
		return None;
	}

	let content = content::temporal_reconciliation_content(loaded, corpus, &selected_ids);
	let selected = SelectedEvidenceText { content, evidence_ids: selected_ids.clone() };
	let evidence = evidence::temporal_reconciliation_evidence(
		evolution,
		&relevant_ids,
		retrieved_evidence_ids,
		&selected_ids,
		ingested,
		loaded,
	);
	let trace_stages =
		trace::temporal_reconciliation_trace_stages(evolution, retrieved_evidence_ids, &evidence);

	Some(TemporalReconciliationSelection { selected, evidence, trace_stages })
}
