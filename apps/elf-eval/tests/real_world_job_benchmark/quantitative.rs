use std::{
	env, fs,
	path::Path,
	process::{self, Command},
};

use color_eyre::{Result, eyre};
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

#[test]
fn quantitative_product_manifest_exports_and_reimports_same_corpus_rows() -> Result<()> {
	let report = support::run_json_report_from(support::adversarial_quality_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-quantitative-product-manifest-test-{}", process::id()));
	let report_path = temp_dir.join("report.json");
	let manifest_path = temp_dir.join("synthetic-rival-product-manifest.json");

	fs::create_dir_all(&temp_dir)?;
	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let export = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-product-manifest")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&manifest_path)
		.arg("--product")
		.arg("Synthetic Rival")
		.arg("--adapter-id")
		.arg("synthetic_rival")
		.arg("--adapter-name")
		.arg("Synthetic Rival adapter")
		.output()?;

	assert!(
		export.status.success(),
		"product manifest export failed: {}",
		String::from_utf8_lossy(&export.stderr)
	);

	let manifest = support::load_json(&manifest_path)?;

	assert_eq!(
		manifest.pointer("/schema").and_then(Value::as_str),
		Some("elf.agent_memory_quantitative_product_manifest/v1")
	);
	assert_eq!(
		manifest.pointer("/rows/0/product").and_then(Value::as_str),
		Some("Synthetic Rival")
	);
	assert_eq!(
		manifest.pointer("/per_query_rows/0/adapter_id").and_then(Value::as_str),
		Some("synthetic_rival")
	);

	let imported = run_report_with_quantitative_manifest(&manifest_path)?;
	let rows = support::array_at(&imported, "/quantitative_scoreboard/rows")?;
	let rival = support::find_by_field(rows, "/adapter_id", "synthetic_rival")?;

	assert_eq!(rows.len(), 2);
	assert_eq!(rival.pointer("/product").and_then(Value::as_str), Some("Synthetic Rival"));
	assert!(!support::array_contains_str(
		&imported,
		"/quantitative_scoreboard/metrics_not_encoded",
		"external_product_manifest_import"
	)?);
	assert!(
		support::array_at(&imported, "/quantitative_scoreboard/per_query_rows")?.iter().any(
			|row| row.pointer("/adapter_id").and_then(Value::as_str) == Some("synthetic_rival")
		)
	);

	Ok(())
}

#[test]
fn quantitative_product_manifest_export_rejects_elf_self_rows() -> Result<()> {
	let report = support::run_json_report_from(support::adversarial_quality_fixture_dir())?;
	let temp_dir = env::temp_dir()
		.join(format!("elf-quantitative-product-manifest-elf-test-{}", process::id()));
	let report_path = temp_dir.join("report.json");
	let manifest_path = temp_dir.join("elf-product-manifest.json");

	fs::create_dir_all(&temp_dir)?;
	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-product-manifest")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&manifest_path)
		.output()?;

	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("exports product ELF"));

	Ok(())
}

#[test]
fn quantitative_product_manifest_rejects_cross_corpus_imports() -> Result<()> {
	let report = support::run_json_report_from(support::adversarial_quality_fixture_dir())?;
	let temp_dir = env::temp_dir()
		.join(format!("elf-quantitative-product-manifest-corpus-test-{}", process::id()));
	let report_path = temp_dir.join("report.json");
	let manifest_path = temp_dir.join("wrong-corpus-product-manifest.json");

	fs::create_dir_all(&temp_dir)?;
	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let export = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-product-manifest")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&manifest_path)
		.arg("--product")
		.arg("Synthetic Rival")
		.arg("--adapter-id")
		.arg("synthetic_rival")
		.arg("--adapter-name")
		.arg("Synthetic Rival adapter")
		.output()?;

	assert!(
		export.status.success(),
		"product manifest export failed: {}",
		String::from_utf8_lossy(&export.stderr)
	);

	let mut manifest = support::load_json(&manifest_path)?;

	support::set_json_pointer(&mut manifest, "/corpus_id", serde_json::json!("wrong-corpus"))?;
	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--quantitative-product-manifest")
		.arg(&manifest_path)
		.output()?;

	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("expected same-corpus"));

	Ok(())
}

#[test]
fn quantitative_product_manifest_rejects_ranked_rows_without_per_query_evidence() -> Result<()> {
	let report = support::run_json_report_from(support::adversarial_quality_fixture_dir())?;
	let temp_dir = env::temp_dir()
		.join(format!("elf-quantitative-product-manifest-per-query-test-{}", process::id()));
	let report_path = temp_dir.join("report.json");
	let manifest_path = temp_dir.join("missing-per-query-product-manifest.json");

	fs::create_dir_all(&temp_dir)?;
	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let export = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-product-manifest")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&manifest_path)
		.arg("--product")
		.arg("Synthetic Rival")
		.arg("--adapter-id")
		.arg("synthetic_rival")
		.arg("--adapter-name")
		.arg("Synthetic Rival adapter")
		.output()?;

	assert!(
		export.status.success(),
		"product manifest export failed: {}",
		String::from_utf8_lossy(&export.stderr)
	);

	let mut manifest = support::load_json(&manifest_path)?;

	support::set_json_pointer(&mut manifest, "/per_query_rows", serde_json::json!([]))?;
	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--quantitative-product-manifest")
		.arg(&manifest_path)
		.output()?;

	assert!(!output.status.success());

	let stderr = String::from_utf8_lossy(&output.stderr);

	assert!(stderr.contains("ranked queries but only 0"));

	Ok(())
}

fn run_report_with_quantitative_manifest(manifest_path: &Path) -> Result<Value> {
	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--quantitative-product-manifest")
		.arg(manifest_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);

	Ok(serde_json::from_slice(&output.stdout)?)
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
