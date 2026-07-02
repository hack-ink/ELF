#[path = "quantitative/audit_manifest.rs"] mod audit_manifest;
#[path = "quantitative/contracts.rs"] mod contracts;
#[path = "quantitative/freshness.rs"] mod freshness;
#[path = "quantitative/metrics.rs"] mod metrics;
#[path = "quantitative/product_manifest.rs"] mod product_manifest;

use std::{path::Path, process::Command};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

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

fn run_report_with_quantitative_audit(manifest_path: &Path, run_id: &str) -> Result<Value> {
	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--run-id")
		.arg(run_id)
		.arg("--quantitative-audit-manifest")
		.arg(manifest_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);

	Ok(serde_json::from_slice(&output.stdout)?)
}
