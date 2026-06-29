use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;

use super::support::*;

#[test]
fn operator_debug_json_report_renders_markdown_links() -> Result<()> {
	let report = run_json_report_from(operator_debug_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-job-operator-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("operator.json");
	let markdown_path = temp_dir.join("operator.md");

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

	assert!(markdown.contains("operator-debug-dropped-evidence-001"));
	assert!(markdown.contains("/viewer?trace_id=11111111-1111-4111-8111-111111111111"));
	assert!(markdown.contains("Raw SQL"));
	assert!(markdown.contains("Replay Candidates"));
	assert!(markdown.contains("Root cause"));

	Ok(())
}

#[test]
fn memory_evolution_report_renders_markdown_counters() -> Result<()> {
	let report = run_json_report_from(evolution_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-memory-evolution-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("evolution-report.json");
	let markdown_path = temp_dir.join("evolution-report.md");

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

	assert!(markdown.contains("## Memory Evolution"));
	assert!(markdown.contains("Temporal validity not encoded: `0`"));
	assert!(markdown.contains("| memory_evolution | memory-evolution-relation-temporal-001"));
	assert!(markdown.contains("`encoded`"));

	Ok(())
}
