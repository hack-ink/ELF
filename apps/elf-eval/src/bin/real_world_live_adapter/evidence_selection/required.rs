use super::*;

pub(super) fn required_evidence_satisfied_impl(
	loaded: &LoadedJob,
	evidence_ids: &[String],
) -> bool {
	if loaded.job.required_evidence.is_empty() {
		return !evidence_ids.is_empty();
	}

	loaded
		.job
		.required_evidence
		.iter()
		.all(|required| evidence_ids.iter().any(|id| id == &required.evidence_id))
}

pub(super) fn selected_required_corpus_texts_impl(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	retrieved_evidence_ids: &[String],
) -> SelectedEvidenceText {
	let required_ids = loaded
		.job
		.required_evidence
		.iter()
		.map(|evidence| evidence.evidence_id.as_str())
		.collect::<BTreeSet<_>>();
	let mut selected_ids = Vec::new();

	if required_ids.is_empty() {
		for evidence_id in retrieved_evidence_ids.iter().take(1) {
			push_unique(&mut selected_ids, evidence_id.clone());
		}
	} else {
		for evidence in &loaded.job.required_evidence {
			if retrieved_evidence_ids.iter().any(|id| id == &evidence.evidence_id) {
				push_unique(&mut selected_ids, evidence.evidence_id.clone());
			}
		}
	}

	let content = selected_ids
		.iter()
		.filter_map(|evidence_id| {
			corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| item.text.clone())
		})
		.collect::<Vec<_>>()
		.join("\n\n");

	SelectedEvidenceText { content, evidence_ids: selected_ids }
}

pub(super) fn live_required_evidence_ids_impl(
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
) -> Vec<String> {
	let mut selected = Vec::new();

	for evidence in &loaded.job.required_evidence {
		if ingested.note_ids_by_evidence.contains_key(&evidence.evidence_id) {
			push_unique(&mut selected, evidence.evidence_id.clone());
		}
	}

	if selected.is_empty() {
		for evidence_id in ingested.note_ids_by_evidence.keys() {
			push_unique(&mut selected, evidence_id.clone());
		}

		selected.sort();
	}

	selected
}

pub(super) fn expected_claim_text_impl(
	loaded: &LoadedJob,
	evidence_ids: &[String],
) -> SelectedEvidenceText {
	let content = loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.map(LiveExpectedClaim::text)
		.collect::<Vec<_>>()
		.join(" ");

	SelectedEvidenceText { content, evidence_ids: evidence_ids.to_vec() }
}

pub(super) fn elf_selected_evidence_text_impl(
	loaded: &LoadedJob,
	stored_corpus: &[CorpusText],
	evidence_ids: &[String],
	ingested: &IngestedCorpus,
	capture_failure: &Option<String>,
) -> (
	SelectedEvidenceText,
	Option<TemporalReconciliationMaterializationEvidence>,
	Option<Vec<TraceStageOutput>>,
) {
	if let Some(failure) = capture_failure {
		return (
			SelectedEvidenceText { content: failure.clone(), evidence_ids: Vec::new() },
			None,
			None,
		);
	}
	if let Some(selection) =
		temporal_reconciliation_selection(loaded, stored_corpus, evidence_ids, ingested)
	{
		return (selection.selected, Some(selection.evidence), Some(selection.trace_stages));
	}

	(selected_required_corpus_texts(loaded, stored_corpus, evidence_ids), None, None)
}
