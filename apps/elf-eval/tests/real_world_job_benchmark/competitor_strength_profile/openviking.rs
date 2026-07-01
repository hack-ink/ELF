use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_openviking_strength_profile(report: &Value) -> Result<()> {
	let openviking_scenarios =
		support::array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")?;
	let trajectory = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-staged-retrieval-trajectory",
	)?;
	let precondition = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-evidence-bearing-retrieval-precondition",
	)?;
	let local_embed_setup = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-local-embed-setup",
	)?;
	let missed_terms = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-missed-expected-terms-evidence",
	)?;
	let hierarchy = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-hierarchy-selection",
	)?;
	let recursive_expansion = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-recursive-context-expansion",
	)?;

	assert_eq!(openviking_scenarios.len(), 6);
	assert_eq!(
		trajectory.pointer("/evidence_class").and_then(Value::as_str),
		Some("fixture_backed")
	);
	assert_eq!(trajectory.pointer("/result_type").and_then(Value::as_str), Some("blocked"));
	assert_eq!(trajectory.pointer("/openviking_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(local_embed_setup.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		local_embed_setup.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(local_embed_setup.pointer("/typed_blocker"), Some(&Value::Null));
	assert_eq!(precondition.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(precondition.pointer("/elf_outcome").and_then(Value::as_str), Some("elf_win"));
	assert_eq!(
		precondition.pointer("/typed_blocker").and_then(Value::as_str),
		Some("output_missed_expected_terms")
	);
	assert_eq!(missed_terms.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(missed_terms.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(hierarchy.pointer("/result_type").and_then(Value::as_str), Some("blocked"));
	assert_eq!(hierarchy.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(
		recursive_expansion.pointer("/result_type").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		recursive_expansion.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);

	Ok(())
}
