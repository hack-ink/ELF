use crate::feature_metrics::{
	BTreeSet, ProducedAnswer, WorkJournalJanitorCandidateArtifact, WorkJournalNextStepArtifact,
	WorkJournalReadbackArtifact, WorkJournalRejectedOptionArtifact,
};

pub(in crate::feature_metrics) fn work_journal_reset_resume_entry_ids(
	answer: &ProducedAnswer,
) -> BTreeSet<&str> {
	answer
		.work_journal_readbacks
		.iter()
		.filter_map(|readback| readback.where_stopped.as_ref())
		.flat_map(|where_stopped| where_stopped.reset_resume_entry_ids.iter().map(String::as_str))
		.collect()
}

pub(in crate::feature_metrics) fn work_journal_decision_rationale_evidence_ids(
	answer: &ProducedAnswer,
) -> BTreeSet<&str> {
	answer
		.work_journal_readbacks
		.iter()
		.filter_map(|readback| readback.where_stopped.as_ref())
		.flat_map(|where_stopped| {
			where_stopped.decision_rationale_evidence_ids.iter().map(String::as_str)
		})
		.collect()
}

pub(in crate::feature_metrics) fn work_journal_rejected_options(
	answer: &ProducedAnswer,
) -> Vec<&WorkJournalRejectedOptionArtifact> {
	answer
		.work_journal_readbacks
		.iter()
		.flat_map(|readback| readback.items.iter())
		.flat_map(|entry| entry.rejected_options.iter())
		.collect()
}

pub(in crate::feature_metrics) fn work_journal_explicit_next_steps(
	answer: &ProducedAnswer,
) -> Vec<&WorkJournalNextStepArtifact> {
	answer
		.work_journal_readbacks
		.iter()
		.flat_map(|readback| readback.items.iter())
		.flat_map(|entry| entry.explicit_next_steps.iter())
		.collect()
}

pub(in crate::feature_metrics) fn work_journal_inferred_next_steps(
	answer: &ProducedAnswer,
) -> Vec<&WorkJournalNextStepArtifact> {
	answer
		.work_journal_readbacks
		.iter()
		.flat_map(|readback| readback.items.iter())
		.flat_map(|entry| entry.inferred_next_steps.iter())
		.collect()
}

pub(in crate::feature_metrics) fn work_journal_handoff_source_refs(
	answer: &ProducedAnswer,
) -> BTreeSet<&str> {
	let mut refs = answer
		.work_journal_readbacks
		.iter()
		.flat_map(|readback| readback.items.iter())
		.flat_map(|entry| entry.source_refs.iter().map(String::as_str))
		.collect::<BTreeSet<_>>();

	for source_ref in answer
		.work_journal_readbacks
		.iter()
		.filter_map(|readback| readback.where_stopped.as_ref())
		.flat_map(|where_stopped| where_stopped.handoff_source_refs.iter().map(String::as_str))
	{
		refs.insert(source_ref);
	}

	refs
}

pub(in crate::feature_metrics) fn work_journal_redacted_marker_ids(
	answer: &ProducedAnswer,
) -> BTreeSet<&str> {
	answer
		.work_journal_readbacks
		.iter()
		.flat_map(|readback| readback.items.iter())
		.flat_map(|entry| entry.redaction_audit.redacted_marker_ids.iter().map(String::as_str))
		.collect()
}

pub(in crate::feature_metrics) fn work_journal_janitor_candidates(
	answer: &ProducedAnswer,
) -> Vec<&WorkJournalJanitorCandidateArtifact> {
	answer
		.work_journal_readbacks
		.iter()
		.flat_map(|readback| readback.janitor_candidates.iter())
		.collect()
}

pub(in crate::feature_metrics) fn work_journal_authority_claim_count(
	readback: &WorkJournalReadbackArtifact,
) -> usize {
	let boundary_claim_count =
		usize::from(readback.promotion_boundary.journal_entry_authority != "source_adjacent_only");
	let missing_promotion_boundary_count = usize::from(
		!readback.promotion_boundary.memory_promotion_required
			&& !readback.promotion_boundary.accepted_refs.is_empty(),
	);
	let where_stopped_claim_count = readback
		.where_stopped
		.as_ref()
		.map_or(0, |where_stopped| where_stopped.journal_only_authority_claims.len());

	boundary_claim_count + missing_promotion_boundary_count + where_stopped_claim_count
}
