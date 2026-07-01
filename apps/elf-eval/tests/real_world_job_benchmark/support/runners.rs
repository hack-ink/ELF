use std::{
	env, fs,
	path::{Path, PathBuf},
	process::{self, Command, Output},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

pub(crate) fn external_adapter_manifest_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("memory_projects_manifest.json")
}

pub(crate) fn run_json_report_from(fixtures: PathBuf) -> Result<Value> {
	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixtures)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	Ok(serde_json::from_slice(&output.stdout)?)
}

pub(crate) fn run_json_report_from_failure(fixtures: PathBuf) -> Result<String> {
	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(fixtures)
		.output()?;

	assert!(
		!output.status.success(),
		"real_world_job runner unexpectedly passed: {}",
		String::from_utf8_lossy(&output.stdout),
	);

	Ok(String::from_utf8_lossy(&output.stderr).to_string())
}

pub(crate) fn run_json_report() -> Result<Value> {
	run_json_report_from(support::fixture_dir())
}

pub(crate) fn run_external_manifest_with_letta_attachment_mutation<F>(
	slug: &str,
	mutation: F,
) -> Result<Output>
where
	F: FnOnce(&mut Value) -> Result<()>,
{
	run_external_manifest_scenario_mutation(
		slug,
		"letta_research_gate",
		"core_block_attachment_readback",
		mutation,
	)
}

pub(crate) fn run_external_manifest_scenario_mutation<F>(
	slug: &str,
	adapter_id: &str,
	scenario_id: &str,
	mutation: F,
) -> Result<Output>
where
	F: FnOnce(&mut Value) -> Result<()>,
{
	let mut manifest =
		serde_json::from_str::<Value>(&fs::read_to_string(external_adapter_manifest_path())?)?;
	let adapters = manifest
		.pointer_mut("/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing manifest adapters"))?;
	let adapter = adapters
		.iter_mut()
		.find(|adapter| adapter.pointer("/adapter_id").and_then(Value::as_str) == Some(adapter_id))
		.ok_or_else(|| eyre::eyre!("missing {adapter_id} adapter"))?;
	let scenarios = adapter
		.pointer_mut("/scenarios")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing {adapter_id} scenarios"))?;
	let scenario = scenarios
		.iter_mut()
		.find(|scenario| {
			scenario.pointer("/scenario_id").and_then(Value::as_str) == Some(scenario_id)
		})
		.ok_or_else(|| eyre::eyre!("missing {scenario_id} scenario"))?;

	mutation(scenario)?;

	let temp_dir = env::temp_dir().join(format!("elf-real-world-{slug}-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let manifest_path = temp_dir.join("memory_projects_manifest.json");

	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	Ok(Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::fixture_dir())
		.arg("--external-adapter-manifest")
		.arg(&manifest_path)
		.output()?)
}
