use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn adversarial_quality_report_exposes_quantitative_scoreboard() -> Result<()> {
	let report = support::run_json_report_from(support::adversarial_quality_fixture_dir())?;

	assert_eq!(
		report.pointer("/quantitative_scoreboard/schema").and_then(Value::as_str),
		Some("elf.agent_memory_quantitative_benchmark/v1")
	);
	assert_eq!(
		report.pointer("/quantitative_scoreboard/generated_at").and_then(Value::as_str),
		report.pointer("/generated_at").and_then(Value::as_str)
	);
	assert_eq!(
		report.pointer("/quantitative_scoreboard/k_values").and_then(Value::as_array),
		Some(&vec![Value::from(1), Value::from(3), Value::from(5), Value::from(10),])
	);
	assert_eq!(
		report
			.pointer("/quantitative_scoreboard/controls/leaderboard_claim_allowed")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report
			.pointer("/quantitative_scoreboard/controls/current_query_count")
			.and_then(Value::as_u64),
		report.pointer("/summary/job_count").and_then(Value::as_u64)
	);

	assert_quantitative_row_contract(&report)?;
	assert_quantitative_per_query_contract(&report)?;

	Ok(())
}

fn assert_quantitative_row_contract(report: &Value) -> Result<()> {
	let rows = support::array_at(report, "/quantitative_scoreboard/rows")?;

	assert_eq!(rows.len(), 1);

	let row = &rows[0];

	assert_eq!(row.pointer("/product").and_then(Value::as_str), Some("ELF"));
	assert_eq!(row.pointer("/adapter_id").and_then(Value::as_str), Some("fixture_smoke"));
	assert_eq!(row.pointer("/suite").and_then(Value::as_str), Some("adversarial_quality"));
	assert_eq!(row.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
	assert_eq!(row.pointer("/result_state").and_then(Value::as_str), Some("pass"));
	assert_eq!(row.pointer("/comparable").and_then(Value::as_bool), Some(true));
	assert_eq!(row.pointer("/metric_comparable").and_then(Value::as_bool), Some(true));
	assert_eq!(row.pointer("/leaderboard_eligible").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/fixture_regression_only").and_then(Value::as_bool), Some(true));
	assert_eq!(row.pointer("/ranking_coverage_state").and_then(Value::as_str), Some("complete"));
	assert_eq!(
		row.pointer("/ranked_candidate_source").and_then(Value::as_str),
		Some("produced_evidence_order")
	);
	assert_eq!(
		row.pointer("/qrel_source").and_then(Value::as_str),
		Some("expected_evidence_fallback")
	);
	assert_eq!(row.pointer("/explicit_qrel_query_count").and_then(Value::as_u64), Some(0));

	for metric in [
		"recall_at_1",
		"precision_at_1",
		"success_at_1",
		"recall_at_5",
		"precision_at_5",
		"success_at_5",
		"mrr",
		"ndcg_at_5",
		"average_precision",
	] {
		assert!(row.pointer(&format!("/metrics/{metric}")).and_then(Value::as_f64).is_some());
		assert_eq!(
			row.pointer(&format!("/metric_states/{metric}")).and_then(Value::as_str),
			Some("pass")
		);
		assert!(row.pointer(&format!("/denominators/{metric}")).and_then(Value::as_u64).is_some());
	}
	for metric in ["recall_at_5", "precision_at_5", "success_at_5"] {
		assert_eq!(
			row.pointer(&format!("/confidence_intervals/{metric}/method")).and_then(Value::as_str),
			Some("wilson_score")
		);
		assert_eq!(
			row.pointer(&format!("/confidence_intervals/{metric}/confidence"))
				.and_then(Value::as_f64),
			Some(0.95)
		);
		assert!(
			row.pointer(&format!("/confidence_intervals/{metric}/denominator"))
				.and_then(Value::as_u64)
				.is_some()
		);
	}

	Ok(())
}

fn assert_quantitative_per_query_contract(report: &Value) -> Result<()> {
	let rows = support::array_at(report, "/quantitative_scoreboard/per_query_rows")?;
	let job_count = report.pointer("/summary/job_count").and_then(Value::as_u64).unwrap_or(0);

	assert_eq!(rows.len() as u64, job_count);

	for row in rows {
		assert_eq!(row.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
		assert_eq!(
			row.pointer("/qrel_source").and_then(Value::as_str),
			Some("expected_evidence_fallback")
		);
		assert!(row.pointer("/candidate_count").and_then(Value::as_u64).is_some());
		assert!(row.pointer("/expected_relevant_count").and_then(Value::as_u64).is_some());
		assert!(row.pointer("/metrics/recall_at_5").is_some());
		assert!(row.pointer("/metrics/precision_at_5").is_some());
		assert!(row.pointer("/metrics/ndcg_at_5").is_some());
		assert!(row.pointer("/metrics/average_precision").is_some());
	}

	Ok(())
}
