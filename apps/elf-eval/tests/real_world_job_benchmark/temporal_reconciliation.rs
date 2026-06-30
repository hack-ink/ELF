use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn live_temporal_reconciliation_report_records_xy905_before_after() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::live_temporal_reconciliation_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::live_temporal_reconciliation_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.live_temporal_reconciliation_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-905"));
	assert_eq!(
		report
			.pointer("/baseline/elf_memory_evolution/job_status_counts/pass")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/baseline/elf_memory_evolution/job_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/post_stage/elf_memory_evolution/job_status_counts/pass")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/post_stage/elf_memory_evolution/job_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/post_stage/elf_memory_evolution/suite_status").and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report.pointer("/post_stage/qmd_memory_evolution/suite_status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		report
			.pointer("/comparison_judgment/current_vs_historical_correctness")
			.and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(
		report
			.pointer("/comparison_judgment/deletion_ttl_tombstone_behavior")
			.and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(support::array_contains_str(
		&report,
		"/trace_contract/answer_fields",
		"selected_historical_evidence"
	)?);
	assert!(support::array_contains_str(
		&report,
		"/trace_contract/materialization_fields",
		"current_winner_evidence_ids"
	)?);
	assert!(support::array_contains_str(
		&report,
		"/trace_contract/trace_stages",
		"temporal_reconciliation.conflict_candidates"
	)?);
	assert!(report.pointer("/trace_contract/negative_gate").and_then(Value::as_str).is_some_and(
		|gate| gate.contains("selected conflict evidence id") && gate.contains("wrong_result")
	));
	assert!(markdown.contains("ELF passing all six memory-evolution jobs"));
	assert!(markdown.contains("selected-but-not-narrated conflicts as `wrong_result`"));
	assert!(markdown.contains("Do not claim ELF beats Graphiti/Zep"));
	assert!(benchmarking_index.contains("2026-06-16-live-temporal-reconciliation-report.md"));
	assert!(
		readme.contains("Live Temporal Reconciliation Report - June 16, 2026")
			&& readme.contains("now reports ELF live `memory_evolution` as 6/6 pass")
	);

	Ok(())
}
