mod controls;
mod imported;
mod input;
mod row;

pub(crate) use self::input::QuantitativeReportInput;

use crate::{
	QuantitativeBenchmarkReport, Result,
	quantitative::{self, QUANTITATIVE_K_VALUES, QUANTITATIVE_SCOREBOARD_SCHEMA},
};

pub(crate) fn quantitative_scoreboard_report(
	input: QuantitativeReportInput<'_>,
) -> Result<QuantitativeBenchmarkReport> {
	let current_row = row::current_quantitative_row(&input)?;
	let imported_rows = imported::imported_quantitative_rows(
		input.product_manifest_path,
		current_row.corpus_id.as_str(),
	)?;
	let mut rows = vec![current_row.row];
	let mut merged_per_query_rows = current_row.per_query_rows;

	rows.extend(imported_rows.rows);
	merged_per_query_rows.extend(imported_rows.per_query_rows);

	let leaderboard_claim_allowed = rows.iter().filter(|row| row.leaderboard_eligible).count() >= 2;
	let controls = controls::quantitative_benchmark_controls(
		&input,
		current_row.ranking_query_count,
		current_row.explicit_qrel_query_count,
		leaderboard_claim_allowed,
	);

	Ok(QuantitativeBenchmarkReport {
		schema: QUANTITATIVE_SCOREBOARD_SCHEMA.to_string(),
		generated_at: input.generated_at.to_string(),
		corpus_id: current_row.corpus_id,
		k_values: QUANTITATIVE_K_VALUES.to_vec(),
		rows,
		per_query_rows: merged_per_query_rows,
		metrics_not_encoded: quantitative::quantitative_metrics_not_encoded(
			imported_rows.row_count,
			imported_rows.per_query_count,
		),
		controls,
		claim_boundary: concat!(
			"Do not convert fixture mechanics, missing explicit qrels, ",
			"or partial candidate coverage into product leaderboard claims."
		)
		.to_string(),
	})
}
