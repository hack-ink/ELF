use crate::markdown::{self, QuantitativeBenchmarkRow, RealWorldReport};

pub(super) fn render_markdown_quantitative_scoreboard(out: &mut String, report: &RealWorldReport) {
	let scoreboard = &report.quantitative_scoreboard;

	if scoreboard.schema.is_empty() {
		return;
	}

	out.push_str("## Quantitative Benchmark Report\n\n");
	out.push_str(concat!(
		"Quantitative rows expose ranking metrics and their claim controls. ",
		"Fixture-backed rows verify benchmark mechanics; leaderboard claims require explicit qrels, ",
		"enough queries, and leakage controls.\n\n"
	));
	out.push_str(&format!("- Schema: `{}`\n", markdown::md_inline(scoreboard.schema.as_str())));
	out.push_str(&format!("- Corpus: `{}`\n", markdown::md_inline(scoreboard.corpus_id.as_str())));
	out.push_str(&format!(
		"- k values: `{}`\n",
		markdown::md_inline(
			scoreboard
				.k_values
				.iter()
				.map(usize::to_string)
				.collect::<Vec<_>>()
				.join(", ")
				.as_str()
		)
	));
	out.push_str(&format!(
		"- Ranking queries: `{}` of `{}`; explicit-qrel queries: `{}`\n",
		scoreboard.controls.current_ranking_query_count,
		scoreboard.controls.current_query_count,
		scoreboard.controls.current_explicit_qrel_query_count
	));
	out.push_str(&format!(
		"- Leaderboard claim allowed: `{}`\n",
		scoreboard.controls.leaderboard_claim_allowed
	));
	out.push_str(&format!(
		"- Claim boundary: {}\n\n",
		markdown::md_cell(scoreboard.claim_boundary.as_str())
	));
	out.push_str("| Product | State | Evidence | Qrels | Sample | Ranking Queries | Recall@5 | ");
	out.push_str("Precision@5 | MRR | nDCG@5 | AP | Leaderboard |\n");
	out.push_str(
		"| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n",
	);

	for row in &scoreboard.rows {
		out.push_str(&format!(
			"| {} | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} | {} | {} | {} | `{}` |\n",
			markdown::md_cell(row.product.as_str()),
			markdown::md_inline(row.result_state.as_str()),
			markdown::md_inline(row.evidence_class.as_str()),
			markdown::md_inline(row.qrel_source.as_str()),
			row.sample_size,
			row.ranking_query_count,
			quantitative_metric(row, "recall_at_5"),
			quantitative_metric(row, "precision_at_5"),
			quantitative_metric(row, "mrr"),
			quantitative_metric(row, "ndcg_at_5"),
			quantitative_metric(row, "average_precision"),
			row.leaderboard_eligible
		));
	}

	if !scoreboard.metrics_not_encoded.is_empty() {
		out.push_str("\nMetrics not encoded:\n");

		for metric in &scoreboard.metrics_not_encoded {
			out.push_str(&format!("- `{}`\n", markdown::md_inline(metric.as_str())));
		}

		out.push('\n');
	}
}

fn quantitative_metric(row: &QuantitativeBenchmarkRow, metric: &str) -> String {
	row.metrics
		.get(metric)
		.and_then(|value| *value)
		.map_or_else(|| "`n/a`".to_string(), |value| format!("`{}`", markdown::round3(value)))
}
