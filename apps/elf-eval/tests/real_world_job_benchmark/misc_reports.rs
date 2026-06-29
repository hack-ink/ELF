use super::*;

#[test]
fn mem0_delete_audit_probe_requires_explicit_delete_history_event() -> Result<()> {
	let script =
		fs::read_to_string(workspace_root()?.join("scripts").join("live-baseline-benchmark.sh"))?;

	assert!(script.contains("def history_has_event"));
	assert!(script.contains("str(entry.get(\"event\", \"\")).upper() == expected"));
	assert!(script.contains(
		"history_has_event(\n        preference_history[\"history\"],\n        \"ADD\","
	));
	assert!(script.contains(
		"history_has_event(\n        preference_history[\"history\"],\n        \"UPDATE\","
	));
	assert!(
		script.contains(
			"history_has_event(\n        delete_history[\"history\"],\n        \"DELETE\","
		)
	);
	assert!(
		!script.contains(
			"contains_terms(\n        delete_history[\"history\"],\n        [\"delete\"],"
		)
	);

	Ok(())
}

#[test]
fn knowledge_json_report_renders_markdown_metrics() -> Result<()> {
	let report = run_json_report_from(knowledge_fixture_dir())?;
	let temp_dir = env::temp_dir().join(format!("elf-real-world-knowledge-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let report_path = temp_dir.join("knowledge-report.json");
	let markdown_path = temp_dir.join("knowledge-report.md");

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

	assert!(markdown.contains("Knowledge Page Metrics"));
	assert!(markdown.contains("Knowledge citation coverage"));
	assert!(markdown.contains("Backlinks: `11` total"));
	assert!(markdown.contains("Unsupported summary count"));
	assert!(markdown.contains("knowledge-project-page-001"));
	assert!(markdown.contains("knowledge-entity-concept-002"));
	assert!(markdown.contains("knowledge-watch-rebuild-003"));

	Ok(())
}
