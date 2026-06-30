use std::{env, fs, process};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn declared_not_encoded_consolidation_jobs_do_not_require_fake_proposals() -> Result<()> {
	let fixture_path =
		support::consolidation_fixture_dir().join("contradiction_report_discard.json");
	let mut fixture = serde_json::from_str::<Value>(&fs::read_to_string(fixture_path)?)?;

	fixture
		.pointer_mut("/corpus/adapter_response")
		.and_then(Value::as_object_mut)
		.ok_or_else(|| eyre::eyre!("missing adapter_response object"))?
		.remove("consolidation");

	let encoding = serde_json::json!({
		"status": "not_encoded",
		"reason": "The qmd live adapter retrieves evidence-linked answers but does not generate or review consolidation proposals."
	});

	fixture
		.as_object_mut()
		.ok_or_else(|| eyre::eyre!("fixture is not an object"))?
		.insert("encoding".to_string(), encoding);

	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-not-encoded-consolidation-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(
		temp_dir.join("not_encoded_consolidation.json"),
		serde_json::to_vec_pretty(&fixture)?,
	)?;

	let report = support::run_json_report_from(temp_dir)?;
	let jobs = support::array_at(&report, "/jobs")?;
	let job =
		support::find_by_field(jobs, "/job_id", "consolidation-contradiction-report-discard-001")?;

	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(1));

	Ok(())
}

#[test]
fn capture_write_policy_live_report_preserves_competitor_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::capture_write_policy_live_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::capture_write_policy_live_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.capture_write_policy_live_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-933"));
	assert_eq!(
		report
			.pointer("/live_capture_results/elf_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report
			.pointer("/live_capture_results/elf_live_real_world/encoded_job_count")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/live_capture_results/elf_live_real_world/redaction_leak_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/live_capture_results/qmd_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("not_encoded")
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let source_binding = support::find_by_field(jobs, "/job_id", "capture-source-id-binding-001")?;
	let source_binding_refs = support::array_at(source_binding, "/runtime_source_refs")?;
	let release_summary_ref =
		support::find_by_field(source_binding_refs, "/evidence_id", "source-id-release-summary")?;

	assert!(support::array_contains_str(
		source_binding,
		"/source_ids",
		"capture:issue-comment-42"
	)?);
	assert_eq!(
		release_summary_ref.pointer("/source_id").and_then(Value::as_str),
		Some("capture:issue-comment-42")
	);
	assert_eq!(
		release_summary_ref.pointer("/evidence_binding").and_then(Value::as_str),
		Some("source_ref")
	);

	let write_policy =
		support::find_by_field(jobs, "/job_id", "capture-write-policy-redaction-001")?;

	assert_eq!(
		write_policy.pointer("/write_policy_redaction_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		write_policy
			.pointer("/runtime_source_refs/0/write_policy_applied")
			.and_then(Value::as_bool),
		Some(true)
	);

	let boundary = support::find_by_field(jobs, "/job_id", "capture-integration-boundaries-001")?;

	assert!(support::array_contains_str(boundary, "/excluded_evidence_ids", "private-span-trap")?);
	assert!(!support::array_contains_str(boundary, "/stored_evidence_ids", "private-span-trap")?);
	assert!(
		support::array_at(boundary, "/runtime_source_refs")?
			.iter()
			.all(|item| item.pointer("/evidence_id").and_then(Value::as_str)
				!= Some("private-span-trap"))
	);

	let positions = support::array_at(&report, "/competitor_positions")?;
	let qmd = support::find_by_field(positions, "/project", "qmd")?;
	let agentmemory = support::find_by_field(positions, "/project", "agentmemory")?;
	let claude_mem = support::find_by_field(positions, "/project", "claude-mem")?;

	assert_eq!(qmd.pointer("/position").and_then(Value::as_str), Some("untested"));
	assert!(qmd.pointer("/reason").and_then(Value::as_str).is_some_and(|reason| {
		reason.contains("typed not_encoded") && reason.contains("ELF self-check")
	}));
	assert_eq!(agentmemory.pointer("/position").and_then(Value::as_str), Some("blocked"));
	assert!(agentmemory.pointer("/reason").and_then(Value::as_str).is_some_and(|reason| {
		reason.contains("process-local StateKV Map") && reason.contains("in-memory index")
	}));
	assert_eq!(claude_mem.pointer("/position").and_then(Value::as_str), Some("blocked"));
	assert!(
		claude_mem
			.pointer("/reason")
			.and_then(Value::as_str)
			.is_some_and(|reason| reason.contains("hooks, timeline, observations")
				&& reason.contains("Docker-contained hook/viewer runner"))
	);

	assert_capture_write_policy_docs(&markdown, &benchmarking_index, &readme);

	Ok(())
}

fn assert_capture_write_policy_docs(markdown: &str, benchmarking_index: &str, readme: &str) {
	assert!(markdown.contains("ELF now has live capture/write-policy self-check evidence"));
	assert!(markdown.contains("not an ELF-over-qmd win"));
	assert!(markdown.contains("| claude-mem capture/viewer flows | `blocked` |"));
	assert!(!markdown.contains("claude-mem capture breadth is untested"));
	assert!(markdown.contains("runtime `source_ref` metadata returned by search"));
	assert!(markdown.contains("Do not claim ELF broadly beats agentmemory or claude-mem"));
	assert!(benchmarking_index.contains("2026-06-11-capture-write-policy-live-report.md"));
	assert!(readme.contains("Capture/Write-Policy Live Report - June 11, 2026"));
	assert!(readme.contains("mem0/OpenMemory"));
	assert!(readme.contains("and memsearch now pass their scoped local baseline"));
	assert!(
		support::collapse_whitespace(readme)
			.contains("claude-mem hook/viewer capture remains blocked until Docker-contained")
	);
}
