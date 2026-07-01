mod boundaries;
mod openviking;
mod qmd;
mod summary_terms;

use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn qmd_openviking_strength_profile_report_preserves_claim_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::strength_profile_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::strength_profile_markdown_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let iteration_direction = fs::read_to_string(support::iteration_direction_report_path()?)?;

	summary_terms::assert_strength_profile_summary(&report);
	summary_terms::assert_strength_profile_terms(&report)?;
	qmd::assert_qmd_strength_profile(&report)?;
	qmd::assert_qmd_wrong_result_diagnosis(&report)?;
	openviking::assert_openviking_strength_profile(&report)?;
	boundaries::assert_strength_profile_json_claim_boundaries(&report)?;
	boundaries::assert_strength_profile_markdown_boundaries(&markdown);
	boundaries::assert_operator_facing_strength_profile_boundaries(
		&readme,
		&benchmarking_index,
		&iteration_direction,
	);

	Ok(())
}
