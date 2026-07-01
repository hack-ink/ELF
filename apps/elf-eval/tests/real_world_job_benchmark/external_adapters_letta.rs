use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_letta_core_archival_gate(adapter: &Value) -> Result<()> {
	assert_eq!(adapter.pointer("/overall_status").and_then(Value::as_str), Some("blocked"));
	assert!(
		adapter
			.pointer("/setup/evidence")
			.and_then(Value::as_str)
			.is_some_and(|evidence| evidence.contains("smoke-letta-core-archive-export-readback")
				&& evidence.contains("Docker-only benchmark-created agent export/readback"))
	);
	assert_eq!(
		adapter.pointer("/setup/command").and_then(Value::as_str),
		Some("cargo make smoke-letta-core-archive-export-readback")
	);
	assert_eq!(
		adapter.pointer("/run/command").and_then(Value::as_str),
		Some(
			"ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 cargo make smoke-letta-core-archive-export-readback"
		)
	);
	assert!(adapter.pointer("/execution_metadata/setup_path").and_then(Value::as_str).is_some_and(
		|setup| setup.contains("exports core block JSON plus archival search/readback JSON")
			&& setup.contains("typed artifact")
	));

	let suites = support::array_at(adapter, "/suites")?;
	let core_suite = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core_suite.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		adapter.pointer("/capabilities/2/capability").and_then(Value::as_str),
		Some("real_world_job_adapter")
	);
	assert_eq!(adapter.pointer("/capabilities/2/status").and_then(Value::as_str), Some("blocked"));

	let scenarios = support::array_at(adapter, "/scenarios")?;
	let attachment =
		support::find_by_field(scenarios, "/scenario_id", "core_block_attachment_readback")?;
	let scope = support::find_by_field(scenarios, "/scenario_id", "core_block_scope_readback")?;
	let provenance =
		support::find_by_field(scenarios, "/scenario_id", "core_block_provenance_readback")?;
	let stale = support::find_by_field(scenarios, "/scenario_id", "stale_core_detection")?;
	let fallback = support::find_by_field(scenarios, "/scenario_id", "archival_fallback_readback")?;
	let decision = support::find_by_field(
		scenarios,
		"/scenario_id",
		"core_archival_project_decision_recovery",
	)?;

	assert_eq!(scenarios.len(), 6);

	for scenario in [attachment, scope, provenance, stale, fallback, decision] {
		assert_eq!(scenario.pointer("/status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/elf_position").and_then(Value::as_str), Some("untested"));
		assert_eq!(
			scenario.pointer("/comparison_outcome").and_then(Value::as_str),
			Some("blocked")
		);
		assert_eq!(
			scenario.pointer("/command").and_then(Value::as_str),
			Some("cargo make smoke-letta-core-archive-export-readback")
		);
		assert_eq!(
			scenario.pointer("/artifact").and_then(Value::as_str),
			Some("tmp/real-world-memory/letta-core-archive/summary.json")
		);
	}

	assert_eq!(attachment.pointer("/comparison_outcome").and_then(Value::as_str), Some("blocked"));
	assert_eq!(stale.pointer("/comparison_outcome").and_then(Value::as_str), Some("blocked"));
	assert_eq!(fallback.pointer("/comparison_outcome").and_then(Value::as_str), Some("blocked"));

	Ok(())
}
