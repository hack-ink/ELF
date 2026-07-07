use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_trace_replay_adoption_json(adoption: &Value) -> Result<()> {
	let local_debug = support::find_by_field(
		support::array_at(adoption, "/scenario_outcomes")?,
		"/scenario_id",
		"local_debug_replay_ux",
	)?;
	let operator_debug = support::find_by_field(
		support::array_at(adoption, "/scenario_outcomes")?,
		"/scenario_id",
		"operator_debugging_viewer_ux",
	)?;

	assert_eq!(local_debug.pointer("/outcome").and_then(Value::as_str), Some("loss"));
	assert!(
		local_debug
			.pointer("/measured_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd stronger on immediate top-10"))
	);
	assert!(support::array_contains_str(
		local_debug,
		"/command_artifacts",
		"docs/evidence/benchmarking/2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"
	)?);
	assert!(support::array_contains_str(
		adoption,
		"/claim_boundaries/not_allowed",
		"Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF memory-system or retrieval-quality win."
	)?);
	assert_eq!(operator_debug.pointer("/outcome").and_then(Value::as_str), Some("win"));
	assert!(
		operator_debug
			.pointer("/measured_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("narrow live operator-debug win over qmd"))
	);
	assert!(support::array_contains_str(
		operator_debug,
		"/command_artifacts",
		"tmp/real-world-job/operator-ux-live-adapters/summary.json"
	)?);
	assert!(support::array_contains_str(
		adoption,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF broadly beats OpenMemory or claude-mem viewer UX from the narrow ELF/qmd operator-debug slice."
	)?);

	Ok(())
}
