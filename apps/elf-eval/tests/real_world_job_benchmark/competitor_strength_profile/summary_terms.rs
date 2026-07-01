use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

pub(crate) fn assert_strength_profile_summary(report: &Value) {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.competitor_strength_profile_report/v1")
	);
	assert_eq!(
		report.pointer("/summary/qmd/retrieval_quality").and_then(Value::as_str),
		Some("tie")
	);
	assert_eq!(
		report.pointer("/summary/qmd/local_query_transparency").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/qmd/local_replayability").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/qmd/overall_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/openviking/overall_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/elf_win")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/qmd_strength_profile/win_tie_loss_summary/tie").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/elf_loss")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/elf_win")
			.and_then(Value::as_u64),
		Some(1)
	);
}

pub(crate) fn assert_strength_profile_terms(report: &Value) -> Result<()> {
	let result_terms = support::array_at(report, "/result_type_terms")?;
	let coverage_terms = support::array_at(report, "/coverage_status_terms")?;
	let outcome_terms = support::array_at(report, "/outcome_terms")?;
	let actual_result_terms = support::string_array_at(report, "/result_type_terms")?;
	let actual_coverage_terms = support::string_array_at(report, "/coverage_status_terms")?;

	assert_eq!(
		actual_result_terms,
		[
			"pass",
			"wrong_result",
			"blocked",
			"incomplete",
			"lifecycle_fail",
			"not_encoded",
			"unsupported_claim",
		]
		.map(str::to_owned)
	);
	assert_eq!(
		actual_coverage_terms,
		[
			"pass",
			"wrong_result",
			"blocked",
			"incomplete",
			"lifecycle_fail",
			"not_encoded",
			"unsupported",
			"unsupported_claim",
		]
		.map(str::to_owned)
	);
	assert!(!result_terms.iter().any(|term| term.as_str() == Some("unsupported")));
	assert!(!result_terms.iter().any(|term| term.as_str() == Some("partial")));
	assert!(!coverage_terms.iter().any(|term| term.as_str() == Some("partial")));
	assert!(result_terms.iter().any(|term| term.as_str() == Some("unsupported_claim")));
	assert!(coverage_terms.iter().any(|term| term.as_str() == Some("unsupported")));

	assert_value_in_terms(report, "/summary/qmd/overall_outcome", outcome_terms)?;
	assert_value_in_terms(report, "/summary/openviking/overall_outcome", outcome_terms)?;

	for scenario in support::array_at(report, "/qmd_strength_profile/scenario_outcomes")? {
		assert_value_in_terms(scenario, "/result_type", result_terms)?;
		assert_value_in_terms(scenario, "/elf_status", coverage_terms)?;
		assert_value_in_terms(scenario, "/qmd_status", coverage_terms)?;
	}
	for scenario in
		support::array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")?
	{
		assert_value_in_terms(scenario, "/result_type", result_terms)?;
		assert_value_in_terms(scenario, "/openviking_status", coverage_terms)?;
		assert_value_in_terms(scenario, "/elf_equivalent_status", coverage_terms)?;
	}

	Ok(())
}

fn assert_value_in_terms(value: &Value, pointer: &str, terms: &[Value]) -> Result<()> {
	let actual = value
		.pointer(pointer)
		.and_then(Value::as_str)
		.ok_or_else(|| eyre::eyre!("missing string at {pointer}"))?;

	assert!(
		terms.iter().any(|term| term.as_str() == Some(actual)),
		"{actual} at {pointer} is not declared in the report term list"
	);

	Ok(())
}
