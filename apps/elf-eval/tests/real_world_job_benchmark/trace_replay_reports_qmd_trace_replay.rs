mod trace_replay_adoption_json;
mod trace_replay_diagnostics_json;
mod trace_replay_markdown_assertions;
mod trace_replay_viewer_boundaries;

use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn qmd_trace_replay_diagnostics_report_preserves_claim_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::trace_replay_diagnostics_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::trace_replay_diagnostics_markdown_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let adoption_report = fs::read_to_string(support::competitor_strength_adoption_report_path()?)?;
	let adoption_json = serde_json::from_str::<Value>(&fs::read_to_string(
		support::competitor_strength_adoption_report_json_path()?,
	)?)?;

	trace_replay_diagnostics_json::assert_trace_replay_diagnostics_json(&report)?;
	trace_replay_markdown_assertions::assert_trace_replay_diagnostics_markdown(&markdown);

	assert!(readme.contains("ELF/qmd Trace Replay Diagnostics Report - June 11, 2026"));
	assert!(benchmarking_index.contains("2026-06-11-elf-qmd-trace-replay-diagnostics-report.md"));
	assert!(benchmarking_index.contains("qmd top-10/replay artifact"));
	assert!(benchmarking_index.contains("ELF trace/admin surfaces"));
	assert!(adoption_report.contains("| Retrieval quality and local debug UX | `loss` |"));
	assert!(adoption_report.contains("Letta scenario rows remain"));
	assert!(adoption_report.contains("blocked or `not_tested`"));

	trace_replay_viewer_boundaries::assert_trace_replay_viewer_blocker_boundaries(
		&readme,
		&markdown,
		&adoption_report,
		&report,
		&adoption_json,
	)?;

	assert!(
		adoption_report
			.contains("Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF")
	);
	assert!(support::array_at(&adoption_json, "/adoption_decision/remaining_caveats")?.iter().any(
		|caveat| {
			caveat.as_str().is_some_and(|text| {
				text.contains("Letta scenario rows remain blocked or not_tested")
			})
		}
	));

	trace_replay_adoption_json::assert_trace_replay_adoption_json(&adoption_json)?;

	Ok(())
}
