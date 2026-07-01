use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;

use crate::support;

#[test]
fn memory_summary_markdown_renders_source_trace_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::memory_summary_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-summary-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("memory-summary-report.json");
	let markdown_path = temp_dir.join("memory-summary-report.md");

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

	assert!(markdown.contains("Memory Summary Metrics"));
	assert!(markdown.contains("memory-summary-source-trace-001"));
	assert!(markdown.contains("Memory summary source-ref coverage"));
	assert!(markdown.contains("Invalid Top-of-Mind"));
	assert!(markdown.contains("Derived Unsupported"));

	Ok(())
}
