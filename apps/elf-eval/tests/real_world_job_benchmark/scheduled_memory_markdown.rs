use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;

use crate::support;

#[test]
fn scheduled_memory_markdown_renders_trace_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::scheduled_memory_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-scheduled-memory-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("scheduled-memory-report.json");
	let markdown_path = temp_dir.join("scheduled-memory-report.md");

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

	assert!(markdown.contains("Scheduled Memory Metrics"));
	assert!(markdown.contains("scheduled-weekly-project-status-summary-001"));
	assert!(markdown.contains("Scheduled memory evidence-ref coverage"));
	assert!(markdown.contains("Trace Coverage"));
	assert!(markdown.contains("Source Mutations"));

	Ok(())
}
