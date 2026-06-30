use crate::scoreboard::ReportSummary;

pub(in crate::scoreboard::elf) fn elf_scoreboard_strengths(summary: &ReportSummary) -> Vec<String> {
	let mut strengths = Vec::new();

	if summary.expected_evidence_recall >= 1.0 {
		strengths.push("Expected evidence recall is complete for encoded jobs.".to_string());
	}
	if summary.source_ref_coverage >= 1.0 {
		strengths
			.push("Source-ref coverage is complete for encoded required evidence.".to_string());
	}
	if summary.stale_answer_count == 0 && summary.stale_retrieval_count == 0 {
		strengths.push("Encoded stale-answer and stale-retrieval counters are zero.".to_string());
	}
	if summary.redaction_leak_count == 0 {
		strengths.push("Encoded redaction leak count is zero.".to_string());
	}
	if summary.work_continuity.is_some() {
		strengths.push("Work Continuity readback metrics are encoded in the report.".to_string());
	}

	strengths
}
