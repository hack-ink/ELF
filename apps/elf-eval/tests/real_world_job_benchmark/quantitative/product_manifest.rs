#[path = "product_manifest/export.rs"] mod export;
#[path = "product_manifest/validation.rs"] mod validation;

use std::{
	env, fs,
	path::PathBuf,
	process::{self, Command},
};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

struct ProductManifestPaths {
	temp_dir: PathBuf,
	report_path: PathBuf,
	manifest_path: PathBuf,
}

fn product_manifest_paths(temp_name: &str, manifest_file: &str) -> ProductManifestPaths {
	let temp_dir = env::temp_dir().join(format!("{temp_name}-{}", process::id()));

	ProductManifestPaths {
		report_path: temp_dir.join("report.json"),
		manifest_path: temp_dir.join(manifest_file),
		temp_dir,
	}
}

fn write_adversarial_report(paths: &ProductManifestPaths) -> Result<()> {
	let report = support::run_json_report_from(support::adversarial_quality_fixture_dir())?;

	fs::create_dir_all(&paths.temp_dir)?;
	fs::write(&paths.report_path, serde_json::to_vec_pretty(&report)?)?;

	Ok(())
}

fn export_synthetic_rival_manifest(paths: &ProductManifestPaths) -> Result<()> {
	write_adversarial_report(paths)?;

	let export = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-product-manifest")
		.arg("--report")
		.arg(&paths.report_path)
		.arg("--out")
		.arg(&paths.manifest_path)
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

	Ok(())
}

fn run_report_with_manifest(paths: &ProductManifestPaths) -> Result<Value> {
	super::run_report_with_quantitative_manifest(&paths.manifest_path)
}
