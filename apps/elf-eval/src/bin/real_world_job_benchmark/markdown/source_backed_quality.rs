use crate::{RealWorldReport, markdown};

pub(super) fn render_markdown_source_backed_quality(out: &mut String, report: &RealWorldReport) {
	let quality = &report.source_backed_quality;

	if quality.schema.is_empty() {
		return;
	}

	out.push_str("## Source-Backed Memory Quality\n\n");
	out.push_str(concat!(
		"This gate reports the source-backed memory quality metrics required by XY-1155. ",
		"Hard-fail leak counters must be zero, and scenario coverage preserves typed ",
		"non-pass states instead of converting missing evidence into wins.\n\n"
	));
	out.push_str(&format!("- Schema: `{}`\n", markdown::md_inline(quality.schema.as_str())));
	out.push_str(&format!(
		"- Result state: `{}`; hard-fail passed: `{}`\n",
		markdown::md_inline(quality.result_state.as_str()),
		quality.hard_fail_passed
	));
	out.push_str(&format!(
		"- Expected evidence recall: `{}`; precision@5: {}; source-ref coverage: `{}`\n",
		markdown::round3(quality.metrics.expected_evidence_recall),
		optional_metric(quality.metrics.precision_at_5),
		markdown::round3(quality.metrics.source_ref_coverage)
	));
	out.push_str(&format!(
		"- Stale suppression: {}; correction persistence: {}; delete/tombstone suppression: {}\n",
		optional_metric(quality.metrics.stale_suppression_rate),
		optional_metric(quality.metrics.correction_persistence_rate),
		optional_metric(quality.metrics.delete_tombstone_suppression_rate)
	));
	out.push_str(&format!(
		"- Unsupported claim rate: `{}`; cross-scope leaks: `{}`; journal-only authority claims: `{}`\n",
		markdown::round3(quality.metrics.unsupported_claim_rate),
		quality.metrics.cross_scope_leak_count,
		quality.metrics.journal_only_authority_claim_count
	));
	out.push_str(&format!(
		"- Context Pack activation precision: {}; recall: {}; trace coverage: {}\n\n",
		optional_metric(quality.metrics.context_pack_activation_precision),
		optional_metric(quality.metrics.context_pack_activation_recall),
		optional_metric(quality.metrics.activation_trace_coverage)
	));
	out.push_str(&format!(
		"- Context Pack decisions: `{}` total, `{}` traced, `{}` incorrect, `{}` enabled expected, `{}` suppressed/disabled/stale/blocked/pinned-ineligible expected\n\n",
		quality.context_pack_decisions.total_decisions,
		quality.context_pack_decisions.traced_decisions,
		quality.context_pack_decisions.incorrect_decisions,
		quality.context_pack_decisions.expected_enabled,
		quality.context_pack_decisions.expected_suppressed
			+ quality.context_pack_decisions.expected_disabled
			+ quality.context_pack_decisions.expected_stale_suppressed
			+ quality.context_pack_decisions.expected_blocked
			+ quality.context_pack_decisions.expected_pinned_ineligible
	));
	out.push_str("| Scenario | State | Jobs | Pass |\n");
	out.push_str("| --- | --- | ---: | ---: |\n");

	for scenario in &quality.scenario_coverage {
		out.push_str(&format!(
			"| {} | `{}` | `{}` | `{}` |\n",
			markdown::md_cell(scenario.scenario.as_str()),
			markdown::md_inline(scenario.status.as_str()),
			scenario.covered_job_count,
			scenario.pass_count
		));
	}

	out.push('\n');
}

fn optional_metric(value: Option<f64>) -> String {
	value.map_or_else(|| "`n/a`".to_string(), |value| format!("`{}`", markdown::round3(value)))
}
