use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn letta_core_archive_export_readback_report_preserves_blocked_gates() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::letta_core_archive_export_readback_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::letta_core_archive_export_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.letta_core_archive_export_readback_summary/v1")
	);
	assert_eq!(
		report.pointer("/adapter_id").and_then(Value::as_str),
		Some("letta_core_archive_export_readback")
	);
	assert_eq!(
		report.pointer("/materialization/status/failure_class").and_then(Value::as_str),
		Some("letta_live_run_disabled")
	);
	assert_eq!(
		report.pointer("/materialization/status/overall").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/status").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/counts/blocked").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/counts/pass").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/scored_benchmark/counts/wrong_result")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/scored_benchmark/evidence_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/materialization/benchmark_input/core_blocks")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(9)
	);
	assert_eq!(
		report
			.pointer("/materialization/benchmark_input/archival_passages")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/materialization/evidence_mapping/expected_evidence_ids")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(14)
	);
	assert_eq!(
		report
			.pointer("/materialization/evidence_mapping/mapped_evidence_ids")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/improvement_regression_readback/judgment")
			.and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(support::array_contains_str(
		&report,
		"/materialization/claim_boundaries/not_allowed",
		"Do not claim ELF beats Letta on core-vs-archival memory from fixture-only ELF evidence."
	)?);
	assert!(markdown.contains("The Letta follow-up is now reproducible"));
	assert!(markdown.contains("6 typed blocked"));
	assert!(markdown.contains("competitive status is unchanged"));
	assert!(benchmarking_index.contains("2026-06-19-letta-core-archive-export-readback-report.md"));
	assert!(readme.contains("Letta core/archive materialization after XY-984"));
	assert!(readme.contains("smoke-letta-core-archive-export-readback"));

	Ok(())
}
