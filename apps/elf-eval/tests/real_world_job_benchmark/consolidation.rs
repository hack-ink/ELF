use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;

use super::support::*;

#[test]
fn consolidation_report_renders_markdown_metrics_and_gaps() -> Result<()> {
	let report = run_json_report_from(consolidation_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-consolidation-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("report.json");
	let markdown_path = temp_dir.join("report.md");

	fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("publish")
		.arg("--report")
		.arg(&report_path)
		.arg("--out")
		.arg(&markdown_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job publisher failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let markdown = fs::read_to_string(markdown_path)?;

	assert!(markdown.contains("## Consolidation"));
	assert!(markdown.contains("Source Mutations"));
	assert!(markdown.contains("Proposal Unsupported Claims"));
	assert!(markdown.contains("Executable Gaps"));
	assert!(markdown.contains("consolidation-contradiction-report-discard-001"));
	assert!(!markdown.contains("live_consolidation_worker_generation"));

	Ok(())
}
