use std::{env, fs, process};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn explicit_qrels_preserve_candidate_order_for_ranking_metrics() -> Result<()> {
	let source_path =
		support::adversarial_quality_fixture_dir().join("conflicting_source_authority.json");
	let mut job = serde_json::from_str::<Value>(&fs::read_to_string(source_path)?)?;

	support::set_json_pointer(
		&mut job,
		"/corpus/adapter_response/answer/evidence_ids",
		serde_json::json!(["old-provider-note", "current-provider-report"]),
	)?;

	job.pointer_mut("/expected_answer")
		.and_then(Value::as_object_mut)
		.ok_or_else(|| eyre::eyre!("missing expected_answer object"))?
		.insert(
			"relevance_judgments".to_string(),
			serde_json::json!([{ "evidence_id": "current-provider-report", "grade": 1.0 }]),
		);

	let temp_dir = env::temp_dir().join(format!("elf-explicit-qrel-order-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("explicit_qrel_order.json"), serde_json::to_vec_pretty(&job)?)?;

	let report = support::run_json_report_from(temp_dir)?;
	let rows = support::array_at(&report, "/quantitative_scoreboard/rows")?;
	let row = rows.first().ok_or_else(|| eyre::eyre!("missing quantitative row"))?;

	assert_eq!(row.pointer("/qrel_source").and_then(Value::as_str), Some("explicit_qrels"));
	assert_eq!(row.pointer("/explicit_qrel_query_count").and_then(Value::as_u64), Some(1));
	assert_eq!(row.pointer("/metrics/recall_at_1").and_then(Value::as_f64), Some(0.0));
	assert_eq!(row.pointer("/metrics/recall_at_3").and_then(Value::as_f64), Some(1.0));
	assert_eq!(row.pointer("/metrics/mrr").and_then(Value::as_f64), Some(0.5));
	assert_eq!(row.pointer("/metrics/average_precision").and_then(Value::as_f64), Some(0.5));
	assert_eq!(row.pointer("/denominators/recall_at_5").and_then(Value::as_u64), Some(1));

	let per_query_rows = support::array_at(&report, "/quantitative_scoreboard/per_query_rows")?;
	let per_query = per_query_rows.first().ok_or_else(|| eyre::eyre!("missing per-query row"))?;

	assert_eq!(per_query.pointer("/qrel_source").and_then(Value::as_str), Some("explicit_qrels"));
	assert_eq!(per_query.pointer("/metrics/mrr").and_then(Value::as_f64), Some(0.5));
	assert_eq!(per_query.pointer("/denominators/recall_at_5").and_then(Value::as_u64), Some(1));

	Ok(())
}
