use crate::markdown::RealWorldReport;

pub(in crate::markdown::header) fn render_markdown_quality_summary(
	out: &mut String,
	report: &RealWorldReport,
) {
	out.push_str(&format!(
		"- Evidence coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.evidence_covered_count,
		report.summary.evidence_required_count,
		report.summary.evidence_coverage
	));
	out.push_str(&format!(
		"- Source-ref coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.source_ref_covered_count,
		report.summary.source_ref_required_count,
		report.summary.source_ref_coverage
	));
	out.push_str(&format!(
		"- Quote coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.quote_covered_count,
		report.summary.quote_required_count,
		report.summary.quote_coverage
	));
	out.push_str(&format!("- Stale retrieval count: `{}`\n", report.summary.stale_retrieval_count));
	out.push_str(&format!(
		"- Scope correctness: `{}/{}` (`{:.3}`), violations `{}`\n",
		report.summary.scope_correct_count,
		report.summary.scope_check_count,
		report.summary.scope_correctness,
		report.summary.scope_violation_count
	));
	out.push_str(&format!("- Redaction leak count: `{}`\n", report.summary.redaction_leak_count));
	out.push_str(&format!(
		"- Qdrant rebuild cases: `{}` encoded, `{}` pass\n",
		report.summary.qdrant_rebuild_case_count, report.summary.qdrant_rebuild_pass_count
	));
	out.push_str(&format!(
		"- Expected evidence recall: `{:.3}` ({}/{})\n",
		report.summary.expected_evidence_recall,
		report.summary.expected_evidence_matched,
		report.summary.expected_evidence_total
	));
	out.push_str(&format!(
		"- Irrelevant context ratio: `{:.3}` ({} irrelevant)\n",
		report.summary.irrelevant_context_ratio, report.summary.irrelevant_context_count
	));
	out.push_str(&format!(
		"- Trace explainability: `{}` job(s), `{}` wrong-result stage attribution(s)\n",
		report.summary.trace_explainability_count,
		report.summary.wrong_result_stage_attribution_count
	));
	out.push_str(&format!(
		"- Consolidation source mutation count: `{}`\n",
		report.summary.consolidation.source_mutation_count
	));
}
