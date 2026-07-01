use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

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

	let imported = super::run_report_with_quantitative_manifest(&manifest_path)?;
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
