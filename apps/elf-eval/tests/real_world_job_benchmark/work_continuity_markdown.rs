use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;

use crate::support;

#[test]
fn work_continuity_markdown_renders_required_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::work_continuity_fixture_dir())?;
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-work-continuity-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("work-continuity-report.json");
	let markdown_path = temp_dir.join("work-continuity-report.md");

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

	assert!(markdown.contains("Work Continuity Metrics"));
	assert!(markdown.contains("work-continuity-redaction-001"));
	assert!(markdown.contains("work-continuity-janitor-false-promotion-001"));
	assert!(markdown.contains("Janitor False Promotion"));
	assert!(markdown.contains("Sensitive Persistence"));
	assert!(markdown.contains("Journal Authority Claims"));
	assert!(markdown.contains("| work-continuity-reset-resume-001 | 1 | 1 | `1/1` (`1.000`)"));
	assert!(markdown.contains(
		"| work-continuity-explicit-next-step-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `1/1` (`1.000`)"
	));
	assert!(markdown.contains(
		"| work-continuity-handoff-source-ref-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`1.000`) | `0/0` (`0.000`) | `1/1` (`1.000`)"
	));
	assert!(markdown.contains(
		"| work-continuity-redaction-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`1.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `1/1` (`1.000`)"
	));
	assert!(markdown.contains(
		"| work-continuity-janitor-false-promotion-001 | 1 | 1 | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`1.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/0` (`0.000`) | `0/1` (`0.000`)"
	));

	Ok(())
}
