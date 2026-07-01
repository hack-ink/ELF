mod json_boundaries;
mod text_boundaries;

use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn current_benchmark_reports_preserve_live_sweep_boundaries() -> Result<()> {
	let measurement_audit = fs::read_to_string(support::measurement_coverage_audit_path()?)?;
	let measurement_audit_json = serde_json::from_str::<Value>(&fs::read_to_string(
		support::measurement_coverage_audit_json_path()?,
	)?)?;
	let competitor_matrix = fs::read_to_string(support::competitor_strength_matrix_path()?)?;
	let competitor_matrix_json = serde_json::from_str::<Value>(&fs::read_to_string(
		support::competitor_strength_matrix_json_path()?,
	)?)?;
	let iteration_direction = fs::read_to_string(support::iteration_direction_report_path()?)?;
	let external_manifest = fs::read_to_string(support::external_adapter_manifest_path())?;
	let comparison_external_projects =
		fs::read_to_string(support::comparison_external_projects_path()?)?;
	let retrieval_debug_profile = serde_json::from_str::<Value>(&fs::read_to_string(
		support::retrieval_debug_profile_json_path()?,
	)?)?;
	let temporal_history = serde_json::from_str::<Value>(&fs::read_to_string(
		support::temporal_history_competitor_gap_json_path()?,
	)?)?;

	text_boundaries::assert_current_report_text_boundaries(
		&measurement_audit,
		&competitor_matrix,
		&iteration_direction,
		&external_manifest,
		&comparison_external_projects,
	);
	text_boundaries::assert_measurement_audit_adapter_status_counts(&measurement_audit);
	json_boundaries::assert_measurement_audit_json(&measurement_audit_json)?;
	json_boundaries::assert_retrieval_debug_profile_json(&retrieval_debug_profile);
	json_boundaries::assert_competitor_strength_matrix_json(&competitor_matrix_json)?;
	json_boundaries::assert_temporal_history_json(&temporal_history)?;

	assert!(competitor_matrix.contains("claude-mem work_resume remains `not_encoded`"));
	assert!(!competitor_matrix.contains("claude-mem `wrong_result`, OpenViking work_resume"));

	Ok(())
}
