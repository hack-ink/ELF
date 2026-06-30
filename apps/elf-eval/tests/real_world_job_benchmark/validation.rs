use color_eyre::{Result, eyre};

use crate::support;

#[test]
fn external_adapter_manifest_rejects_unmeasured_win_loss_scenario_outcomes() -> Result<()> {
	let output = support::run_external_manifest_with_letta_attachment_mutation(
		"invalid-scenario-outcome-test",
		|scenario| {
			support::set_json_pointer(scenario, "/status", serde_json::json!("not_encoded"))?;

			support::set_json_pointer(scenario, "/comparison_outcome", serde_json::json!("win"))
		},
	)?;

	assert!(!output.status.success(), "invalid scenario outcome unexpectedly passed");
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("not_encoded status with win outcome")
	);

	Ok(())
}

#[test]
fn external_adapter_manifest_rejects_unmeasured_win_loss_scenario_positions() -> Result<()> {
	let output = support::run_external_manifest_with_letta_attachment_mutation(
		"invalid-scenario-position-test",
		|scenario| {
			support::set_json_pointer(scenario, "/status", serde_json::json!("not_encoded"))?;
			support::set_json_pointer(scenario, "/elf_position", serde_json::json!("wins"))?;

			support::set_json_pointer(
				scenario,
				"/comparison_outcome",
				serde_json::json!("not_tested"),
			)
		},
	)?;

	assert!(!output.status.success(), "invalid scenario position unexpectedly passed");
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("not_encoded status with wins position")
	);

	Ok(())
}

#[test]
fn external_adapter_manifest_rejects_blocked_status_without_blocked_outcome() -> Result<()> {
	let output = support::run_external_manifest_scenario_mutation(
		"invalid-blocked-scenario-outcome-test",
		"letta_research_gate",
		"stale_core_detection",
		|scenario| {
			scenario
				.as_object_mut()
				.ok_or_else(|| eyre::eyre!("scenario is not an object"))?
				.remove("comparison_outcome");

			Ok(())
		},
	)?;

	assert!(!output.status.success(), "invalid blocked scenario unexpectedly passed");
	assert!(
		String::from_utf8_lossy(&output.stderr)
			.contains("blocked status without blocked comparison outcome")
	);

	Ok(())
}

#[test]
fn external_adapter_manifest_rejects_conflicting_scenario_position_and_outcome() -> Result<()> {
	let output = support::run_external_manifest_with_letta_attachment_mutation(
		"invalid-scenario-position-outcome-test",
		|scenario| {
			support::set_json_pointer(scenario, "/status", serde_json::json!("pass"))?;
			support::set_json_pointer(scenario, "/elf_position", serde_json::json!("ties"))?;

			support::set_json_pointer(scenario, "/comparison_outcome", serde_json::json!("loss"))
		},
	)?;

	assert!(!output.status.success(), "conflicting scenario unexpectedly passed");
	assert!(String::from_utf8_lossy(&output.stderr).contains("ties position with loss outcome"));

	Ok(())
}
