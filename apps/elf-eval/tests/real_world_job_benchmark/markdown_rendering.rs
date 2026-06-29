use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

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

#[test]
fn external_adapter_markdown_renders_nonzero_scenario_losses() -> Result<()> {
	let mut report = support::run_json_report()?;
	let adapters = report
		.pointer_mut("/external_adapters/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing external adapter records"))?;
	let adapter = adapters
		.iter_mut()
		.find(|adapter| {
			adapter.pointer("/adapter_id").and_then(Value::as_str)
				== Some("agentmemory_live_baseline")
		})
		.ok_or_else(|| eyre::eyre!("missing agentmemory adapter"))?;

	support::set_json_pointer(adapter, "/scenarios/0/elf_position", serde_json::json!("loses"))?;
	support::set_json_pointer(
		adapter,
		"/scenarios/0/comparison_outcome",
		serde_json::json!("loss"),
	)?;
	support::set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_position_counts",
		serde_json::json!({
			"wins": 2,
			"ties": 4,
			"loses": 2,
			"untested": 10
		}),
	)?;
	support::set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_outcome_counts",
		serde_json::json!({
			"win": 2,
			"tie": 4,
			"loss": 2,
			"not_tested": 7,
			"blocked": 1,
			"non_goal": 2
		}),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-loss-scenario-test-{}", process::id()));

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

	assert!(markdown.contains("ELF scenario positions: `wins=2, ties=4, loses=2, untested=10`"));
	assert!(markdown.contains(
		"Scenario comparison outcomes: `win=2, tie=4, loss=2, not_tested=7, blocked=1, non_goal=2`"
	));
	assert!(markdown.contains(
		"| `agentmemory_live_baseline` | `basic_same_corpus_retrieval` | `retrieval` | `pass` | `loss` |"
	));

	Ok(())
}

#[test]
fn external_adapter_markdown_omits_scenario_summary_when_manifest_has_no_scenarios() -> Result<()> {
	let mut report = support::run_json_report()?;
	let adapters = report
		.pointer_mut("/external_adapters/adapters")
		.and_then(Value::as_array_mut)
		.ok_or_else(|| eyre::eyre!("missing external adapter records"))?;

	for adapter in adapters {
		support::set_json_pointer(adapter, "/scenarios", serde_json::json!([]))?;
	}

	support::set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_status_counts",
		serde_json::json!({
			"real": 0,
			"mocked": 0,
			"unsupported": 0,
			"blocked": 0,
			"incomplete": 0,
			"wrong_result": 0,
			"lifecycle_fail": 0,
			"pass": 0,
			"not_encoded": 0
		}),
	)?;
	support::set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_position_counts",
		serde_json::json!({
			"wins": 0,
			"ties": 0,
			"loses": 0,
			"untested": 0
		}),
	)?;
	support::set_json_pointer(
		&mut report,
		"/external_adapters/summary/scenario_outcome_counts",
		serde_json::json!({
			"win": 0,
			"tie": 0,
			"loss": 0,
			"not_tested": 0,
			"blocked": 0,
			"non_goal": 0
		}),
	)?;

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-no-scenario-test-{}", process::id()));

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

	assert!(markdown.contains("External Adapter Coverage"));
	assert!(!markdown.contains("Scenario coverage statuses:"));
	assert!(!markdown.contains("ELF scenario positions:"));
	assert!(!markdown.contains("Scenario comparison outcomes:"));
	assert!(!markdown.contains("### Adapter Scenario Judgments"));

	Ok(())
}
