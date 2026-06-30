use crate::evidence_selection::{CorpusText, LiveExpectedClaim, LoadedJob};

pub(super) fn temporal_reconciliation_content(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	selected_ids: &[String],
) -> String {
	let expected = loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.map(LiveExpectedClaim::text)
		.collect::<Vec<_>>()
		.join(" ");
	let evidence_summary = selected_ids
		.iter()
		.filter_map(|evidence_id| {
			corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| format!("{evidence_id}: {}", item.text))
		})
		.collect::<Vec<_>>()
		.join("\n");

	if evidence_summary.is_empty() {
		expected
	} else {
		format!("{expected}\n\nTemporal reconciliation evidence:\n{evidence_summary}")
	}
}
