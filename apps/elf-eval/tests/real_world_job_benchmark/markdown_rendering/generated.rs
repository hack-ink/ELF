use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;

use crate::support;

#[test]
fn generated_json_report_renders_markdown() -> Result<()> {
	let report = support::run_json_report()?;
	let temp_dir = env::temp_dir().join(format!("elf-real-world-job-test-{}", process::id()));

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

	assert!(markdown.contains("# Real-World Job Benchmark Report"));
	assert!(markdown.contains("work_resume"));
	assert!(markdown.contains("Capture And Integration Coverage"));
	assert!(markdown.contains("Quantitative Benchmark Report"));
	assert!(markdown.contains("leaderboard claims require explicit qrels"));
	assert!(markdown.contains("| ELF | `pass` | `fixture_backed`"));
	assert!(markdown.contains("External Adapter Coverage"));
	assert!(markdown.contains("live-baseline-only"));
	assert!(markdown.contains("live real-world"));
	assert!(markdown.contains("does not convert live-baseline retrieval results"));
	assert!(markdown.contains("fixture-backed"));
	assert!(markdown.contains("Answer Type"));
	assert!(markdown.contains("Caveat Required"));
	assert!(markdown.contains("Refusal Required"));
	assert!(markdown.contains("agentmemory-style hook capture"));
	assert!(markdown.contains("xy844-current-worktree"));
	assert!(markdown.contains("Existing live-baseline reports remain valid"));
	assert!(markdown.contains("### Adapter Scenario Judgments"));
	assert!(markdown.contains("ELF scenario positions: `wins=10, ties=11, loses=1, untested=53`"));
	assert!(markdown.contains(
		"Scenario comparison outcomes: `win=10, tie=11, loss=1, not_tested=19, blocked=29, non_goal=5`"
	));
	assert!(markdown.contains("| `claude_mem_live_baseline` | `same_corpus_retrieval`"));
	assert!(markdown.contains("| `memsearch_live_baseline` | `ttl_expiry_lifecycle`"));

	Ok(())
}
