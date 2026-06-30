mod claims;
mod common;
mod required;
mod temporal;

use crate::{
	BTreeSet, CorpusText, IngestedCorpus, LiveExpectedClaim, LiveMemoryEvolution, LoadedJob,
	SelectedEvidenceText, TemporalReconciliationMaterializationEvidence,
	TemporalReconciliationSelection, TraceStageOutput, Value, push_unique, serde_json,
};

pub(super) fn answer_claims(loaded: &LoadedJob, evidence_ids: &[String]) -> Vec<Value> {
	claims::answer_claims_impl(loaded, evidence_ids)
}

pub(super) fn required_evidence_satisfied(loaded: &LoadedJob, evidence_ids: &[String]) -> bool {
	required::required_evidence_satisfied_impl(loaded, evidence_ids)
}

pub(super) fn selected_required_corpus_texts(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	retrieved_evidence_ids: &[String],
) -> SelectedEvidenceText {
	required::selected_required_corpus_texts_impl(loaded, corpus, retrieved_evidence_ids)
}

pub(super) fn live_required_evidence_ids(
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
) -> Vec<String> {
	required::live_required_evidence_ids_impl(loaded, ingested)
}

pub(super) fn expected_claim_text(
	loaded: &LoadedJob,
	evidence_ids: &[String],
) -> SelectedEvidenceText {
	required::expected_claim_text_impl(loaded, evidence_ids)
}

pub(super) fn elf_selected_evidence_text(
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
	required::elf_selected_evidence_text_impl(
		loaded,
		stored_corpus,
		evidence_ids,
		ingested,
		capture_failure,
	)
}

fn temporal_reconciliation_claims(loaded: &LoadedJob, evidence_ids: &[String]) -> Vec<Value> {
	claims::temporal_reconciliation_claims_impl(loaded, evidence_ids)
}

fn temporal_reconciliation_selection(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	retrieved_evidence_ids: &[String],
	ingested: &IngestedCorpus,
) -> Option<TemporalReconciliationSelection> {
	temporal::temporal_reconciliation_selection_impl(
		loaded,
		corpus,
		retrieved_evidence_ids,
		ingested,
	)
}
