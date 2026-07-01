use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn openmemory_ui_export_product_recheck_preserves_blocked_boundary() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::openmemory_ui_export_product_readback_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::openmemory_ui_export_product_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.openmemory_ui_export_product_recheck_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-987"));
	assert_eq!(
		report.pointer("/command/command").and_then(Value::as_str),
		Some("cargo make openmemory-ui-export-readback")
	);
	assert_eq!(report.pointer("/command/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		report.pointer("/command/probe_artifact").and_then(Value::as_str),
		Some("tmp/live-baseline/mem0-openmemory-ui-export.json")
	);
	assert_eq!(report.pointer("/run/sdk_check_summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/run/ui_export_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		report.pointer("/run/ui_export_reason_code").and_then(Value::as_str),
		Some("DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER")
	);
	assert_eq!(
		report
			.pointer("/same_corpus_boundary/sdk_get_all_is_ui_export_evidence")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report
			.pointer("/openmemory_product_surface/export_requires_running_container")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert!(
		report
			.pointer("/openmemory_probe/attempt/output_excerpt")
			.and_then(Value::as_str)
			.is_some_and(|excerpt| excerpt.contains("docker: command not found")
				&& excerpt.contains("Container 'openmemory-openmemory-mcp-1' not found/running"))
	);
	assert_eq!(
		report.pointer("/classification/comparison_judgment").and_then(Value::as_str),
		Some("unchanged")
	);
	assert_eq!(
		report
			.pointer("/claim_boundary/product_browser_or_dashboard_readback_reached")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert!(support::array_contains_str(
		&report,
		"/improvement_regression_readback/unchanged",
		"OpenMemory product UI/export readback remains blocked before same-corpus product app database validation."
	)?);
	assert!(support::array_contains_str(
		&report,
		"/next_optimization_direction/required_fields",
		"same_corpus_import_into_openmemory_app_database"
	)?);
	assert!(markdown.contains("OpenMemory UI/export product-readback status is unchanged"));
	assert!(markdown.contains("Product browser/dashboard readback reached"));
	assert!(
		benchmarking_index.contains("2026-06-19-openmemory-ui-export-product-readback-report.md")
	);
	assert!(readme.contains("OpenMemory UI/Export Product Readback Report - June 19, 2026"));
	assert!(readme.contains("OpenMemory UI/export product recheck after XY-987"));

	Ok(())
}
