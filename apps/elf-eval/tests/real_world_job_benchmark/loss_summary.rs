use std::{
	env, fs,
	path::Path,
	process::{self, Command},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn external_adapter_run_summarizes_nonzero_scenario_losses() -> Result<()> {
	let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("real_world_external_adapters")
		.join("memory_projects_manifest.json");
	let mut manifest = serde_json::from_str::<Value>(&fs::read_to_string(manifest_path)?)?;
	let adapters = manifest
		.pointer_mut("/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing manifest adapters"))?;
	let adapter = adapters
		.iter_mut()
		.find(|adapter| {
			adapter.pointer("/adapter_id").and_then(Value::as_str)
				== Some("agentmemory_live_baseline")
		})
		.ok_or_else(|| eyre::eyre!("missing agentmemory adapter"))?;

	support::set_json_pointer(adapter, "/scenarios/0/elf_position", serde_json::json!("loses"))?;
	support::set_json_pointer(
		adapter,
		"/scenarios/0/comparison_outcome",
		serde_json::json!("loss"),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-loss-manifest-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let manifest_path = temp_dir.join("memory_projects_manifest.json");

	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::fixture_dir())
		.arg("--external-adapter-manifest")
		.arg(&manifest_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let report = serde_json::from_slice::<Value>(&output.stdout)?;

	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/loses")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_position_counts/untested")
			.and_then(Value::as_u64),
		Some(52)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/loss")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/external_adapters/summary/scenario_outcome_counts/not_tested")
			.and_then(Value::as_u64),
		Some(18)
	);

	let adapters = support::array_at(&report, "/external_adapters/adapters")?;
	let agentmemory = support::find_by_field(adapters, "/adapter_id", "agentmemory_live_baseline")?;

	assert_eq!(
		agentmemory.pointer("/scenarios/0/elf_position").and_then(Value::as_str),
		Some("loses")
	);

	Ok(())
}
