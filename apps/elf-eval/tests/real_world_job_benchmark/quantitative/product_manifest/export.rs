use std::process::Command;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn quantitative_product_manifest_exports_and_reimports_same_corpus_rows() -> Result<()> {
	let paths = super::product_manifest_paths(
		"elf-quantitative-product-manifest-test",
		"synthetic-rival-product-manifest.json",
	);

	super::export_synthetic_rival_manifest(&paths)?;

	let manifest = support::load_json(&paths.manifest_path)?;

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

	let imported = super::run_report_with_manifest(&paths)?;
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
	let paths = super::product_manifest_paths(
		"elf-quantitative-product-manifest-elf-test",
		"elf-product-manifest.json",
	);

	super::write_adversarial_report(&paths)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-product-manifest")
		.arg("--report")
		.arg(&paths.report_path)
		.arg("--out")
		.arg(&paths.manifest_path)
		.output()?;

	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("exports product ELF"));

	Ok(())
}
