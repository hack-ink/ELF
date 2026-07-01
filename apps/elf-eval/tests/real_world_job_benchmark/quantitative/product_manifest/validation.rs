use std::{fs, process::Command};

use color_eyre::Result;

use crate::support;

#[test]
fn quantitative_product_manifest_rejects_cross_corpus_imports() -> Result<()> {
	let paths = super::product_manifest_paths(
		"elf-quantitative-product-manifest-corpus-test",
		"wrong-corpus-product-manifest.json",
	);

	super::export_synthetic_rival_manifest(&paths)?;

	let mut manifest = support::load_json(&paths.manifest_path)?;

	support::set_json_pointer(&mut manifest, "/corpus_id", serde_json::json!("wrong-corpus"))?;
	fs::write(&paths.manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--quantitative-product-manifest")
		.arg(&paths.manifest_path)
		.output()?;

	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("expected same-corpus"));

	Ok(())
}

#[test]
fn quantitative_product_manifest_rejects_ranked_rows_without_per_query_evidence() -> Result<()> {
	let paths = super::product_manifest_paths(
		"elf-quantitative-product-manifest-per-query-test",
		"missing-per-query-product-manifest.json",
	);

	super::export_synthetic_rival_manifest(&paths)?;

	let mut manifest = support::load_json(&paths.manifest_path)?;

	support::set_json_pointer(&mut manifest, "/per_query_rows", serde_json::json!([]))?;
	fs::write(&paths.manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--quantitative-product-manifest")
		.arg(&paths.manifest_path)
		.output()?;

	assert!(!output.status.success());

	let stderr = String::from_utf8_lossy(&output.stderr);

	assert!(stderr.contains("ranked queries but only 0"));

	Ok(())
}
