use super::*;

pub(super) fn render_markdown_scoreboard(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Quality Scoreboard Grammar\n\n");
	out.push_str("The scoreboard is a claim grammar, not a leaderboard. A report may claim only the statuses and evidence classes represented by its source JSON.\n\n");
	out.push_str(&format!("- Schema: `{}`\n", md_inline(report.scoreboard.schema.as_str())));
	out.push_str(&format!(
		"- Result states: `{}`\n",
		md_inline(report.scoreboard.result_states.join(", ").as_str())
	));
	out.push_str(&format!(
		"- Evidence classes: `{}`\n",
		md_inline(report.scoreboard.evidence_classes.join(", ").as_str())
	));
	out.push_str(&format!(
		"- Metric basis: `{}` at k=`{}`\n",
		md_inline(report.scoreboard.metric_basis.as_str()),
		report.scoreboard.retrieval_k
	));
	out.push_str(&format!(
		"- Summary claim: `{}`\n",
		md_inline(report.scoreboard.summary_claim.as_str())
	));
	out.push_str(&format!(
		"- Job summary claim: `{}`\n",
		md_inline(report.scoreboard.job_summary_claim.as_str())
	));
	out.push_str(&format!(
		"- Job typed non-pass rows: `{}` ({})\n",
		report.scoreboard.job_typed_non_pass_count,
		md_inline(
			scoreboard_state_list(&report.scoreboard.job_typed_non_pass_states_present).as_str()
		)
	));
	out.push_str(&format!(
		"- External-adapter typed non-pass rows: `{}` ({})\n",
		report.scoreboard.external_adapter_typed_non_pass_count,
		md_inline(
			scoreboard_state_list(
				&report.scoreboard.external_adapter_typed_non_pass_states_present
			)
			.as_str()
		)
	));
	out.push_str(&format!(
		"- Typed non-pass rows: `{}` ({})\n",
		report.scoreboard.typed_non_pass_count,
		md_inline(scoreboard_state_list(&report.scoreboard.typed_non_pass_states_present).as_str())
	));
	out.push_str(&format!(
		"- Evidence class counts: `{}`\n",
		md_inline(scoreboard_evidence_class_count_display(&report.scoreboard).as_str())
	));
	out.push_str(&format!(
		"- Unqualified win claim allowed: `{}`\n",
		report.scoreboard.unqualified_win_claim_allowed
	));
	out.push_str(&format!(
		"- Claim boundary: {}\n\n",
		md_cell(report.scoreboard.claim_boundary.as_str())
	));
	out.push_str("| Product | State | Evidence | Comparable | Runtime Gates | Recall@k | Precision@k | MRR | nDCG | Stale Suppression | Update/Delete | Source Refs | Latency | Next Evidence |\n");
	out.push_str(
		"| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | --- | --- |\n",
	);

	for row in &report.scoreboard.rows {
		out.push_str(&format!(
			"| {} | `{}` | `{}` | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
			md_cell(row.product_name.as_str()),
			md_inline(row.result_state.as_str()),
			md_inline(row.evidence_class.as_str()),
			row.comparable,
			scoreboard_runtime_gate_cell(row),
			scoreboard_optional_f64(row.metrics.retrieval.recall_at_k),
			scoreboard_optional_f64(row.metrics.retrieval.precision_at_k),
			scoreboard_optional_f64(row.metrics.retrieval.mrr),
			scoreboard_optional_f64(row.metrics.retrieval.ndcg),
			scoreboard_optional_f64(row.metrics.lifecycle.stale_suppression),
			scoreboard_update_delete_cell(row),
			scoreboard_optional_f64(row.metrics.coverage.source_ref_coverage),
			scoreboard_latency_cell(row),
			md_cell(scoreboard_list_cell(&row.next_evidence).as_str())
		));
	}

	if !report.scoreboard.optimization_roadmap.is_empty() {
		out.push_str("\nOptimization direction:\n");

		for item in &report.scoreboard.optimization_roadmap {
			out.push_str(&format!("- {}\n", md_cell(item.as_str())));
		}

		out.push('\n');
	}
}

fn scoreboard_state_list(states: &[String]) -> String {
	if states.is_empty() { "none".to_string() } else { states.join(", ") }
}

fn scoreboard_evidence_class_count_display(scoreboard: &ScoreboardReport) -> String {
	SCOREBOARD_EVIDENCE_CLASSES
		.iter()
		.map(|state| {
			let count = scoreboard.evidence_class_counts.get(*state).copied().unwrap_or_default();

			format!("{state}={count}")
		})
		.collect::<Vec<_>>()
		.join(", ")
}

fn scoreboard_optional_f64(value: Option<f64>) -> String {
	value.map_or_else(|| "`n/a`".to_string(), |value| format!("`{}`", round3(value)))
}

fn scoreboard_optional_f64_plain(value: Option<f64>) -> String {
	value.map_or_else(|| "n/a".to_string(), |value| round3(value).to_string())
}

fn scoreboard_runtime_gate_cell(row: &ScoreboardRow) -> String {
	format!(
		"`same_corpus={}`<br>`source_ids={}`<br>`held_out={}`<br>`leakage={}`<br>`runtime={}`<br>`digest={}`",
		row.same_corpus,
		row.source_id_mapped,
		row.held_out,
		row.leakage_audited,
		row.product_runtime,
		row.container_digest_identified
	)
}

fn scoreboard_update_delete_cell(row: &ScoreboardRow) -> String {
	format!(
		"`update={}`<br>`delete={}`",
		scoreboard_optional_f64_plain(row.metrics.lifecycle.update_correctness),
		scoreboard_optional_f64_plain(row.metrics.lifecycle.delete_correctness)
	)
}

fn scoreboard_latency_cell(row: &ScoreboardRow) -> String {
	row.metrics
		.operations
		.mean_latency_ms
		.map_or_else(|| "`n/a`".to_string(), |latency| format!("`{} ms`", round3(latency)))
}

fn scoreboard_list_cell(values: &[String]) -> String {
	if values.is_empty() { "none".to_string() } else { values.join("; ") }
}
