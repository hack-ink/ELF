use std::fs;

use color_eyre::Result;

use crate::support;

#[test]
fn final_source_backed_closeout_report_covers_xy1157_review_surface() -> Result<()> {
	let report = fs::read_to_string(
		support::final_source_backed_project_memory_closeout_report_markdown_path()?,
	)?;

	for required in [
		"Source Library",
		"Memory Authority",
		"Source-to-Memory Authority loop",
		"Knowledge Workspace",
		"Work Journal",
		"Dreaming Review",
		"Context Pack v1",
		"Automatic Context Routing",
		"Recall Engine",
		"Recall Debug",
		"benchmark validity",
		"Competitor And Unsupported Claim Boundaries",
		"Decodex Status Accuracy",
		"`decodex status --json`",
		"Any P0 or P1 finding in those areas remains a blocker",
	] {
		assert!(report.contains(required), "missing closeout coverage for {required}");
	}

	Ok(())
}

#[test]
fn final_source_backed_closeout_report_preserves_claim_boundaries_and_docs_links() -> Result<()> {
	let report = fs::read_to_string(
		support::final_source_backed_project_memory_closeout_report_markdown_path()?,
	)?;
	let index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	for boundary in [
		"no universal leaderboard",
		"no broad \"ELF beats every competitor\" claim",
		"no private-corpus or provider-backed production quality claim",
		"qmd still has a short local replay/debug ergonomics edge",
		"Missing, blocked, incomplete, wrong-result, not-tested, public-proxy, local fixture",
	] {
		assert!(report.contains(boundary), "missing claim boundary {boundary}");
	}

	assert!(index.contains("2026-07-03-final-source-backed-project-memory-closeout-report.md"));
	assert!(readme.contains("Final Source-Backed Project Memory Closeout Report - July 3, 2026"));
	assert!(readme.contains("Latest real-world benchmark report: July 3, 2026"));

	Ok(())
}
