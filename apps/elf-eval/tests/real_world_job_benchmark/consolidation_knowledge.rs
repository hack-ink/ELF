use std::{env, fs, path::Path, process};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

fn real_world_live_adapter_sources(workspace: &Path) -> Result<String> {
	let mut source = fs::read_to_string(
		workspace.join("apps/elf-eval/src/bin/real_world_live_adapter/main.rs"),
	)?;

	append_rust_sources(
		workspace.join("apps/elf-eval/src/bin/real_world_live_adapter").as_path(),
		&mut source,
	)?;

	Ok(source)
}

fn real_world_job_benchmark_sources(workspace: &Path) -> Result<String> {
	let mut source = fs::read_to_string(
		workspace.join("apps/elf-eval/src/bin/real_world_job_benchmark/main.rs"),
	)?;

	append_rust_sources(
		workspace.join("apps/elf-eval/src/bin/real_world_job_benchmark").as_path(),
		&mut source,
	)?;

	Ok(source)
}

fn append_rust_sources(dir: &Path, source: &mut String) -> Result<()> {
	let mut entries = Vec::new();

	for entry in fs::read_dir(dir)? {
		entries.push(entry?.path());
	}

	entries.sort();

	for path in entries {
		if path.is_dir() {
			append_rust_sources(path.as_path(), source)?;
		} else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
			source.push('\n');
			source.push_str(fs::read_to_string(path)?.as_str());
		}
	}

	Ok(())
}

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

#[test]
fn live_consolidation_report_preserves_reviewable_output_boundaries() -> Result<()> {
	let workspace = support::workspace_root()?;
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::live_consolidation_proposal_scoring_report_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::live_consolidation_proposal_scoring_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let benchmark_runbook = fs::read_to_string(
		workspace
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("real_world_agent_memory_benchmark.md"),
	)?;
	let makefile = fs::read_to_string(workspace.join("Makefile.toml"))?;
	let live_script =
		fs::read_to_string(workspace.join("scripts/real-world-consolidation-live-adapter.sh"))?;
	let live_adapter = real_world_live_adapter_sources(&workspace)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.live_consolidation_proposal_scoring_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-934"));
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/encoded_job_count")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/proposal_count")
			.and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/source_mutation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/elf_live_real_world/review_event_count")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/live_consolidation_results/qmd_live_real_world/suite_status")
			.and_then(Value::as_str),
		Some("not_encoded")
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let project_summary =
		support::find_by_field(jobs, "/job_id", "consolidation-project-summary-apply-001")?;
	let preference =
		support::find_by_field(jobs, "/job_id", "consolidation-preference-candidate-defer-001")?;
	let contradiction =
		support::find_by_field(jobs, "/job_id", "consolidation-contradiction-report-discard-001")?;

	assert_eq!(
		project_summary.pointer("/final_review_state").and_then(Value::as_str),
		Some("applied")
	);
	assert_eq!(project_summary.pointer("/review_event_count").and_then(Value::as_u64), Some(2));
	assert_eq!(preference.pointer("/final_review_state").and_then(Value::as_str), Some("archived"));
	assert_eq!(
		contradiction.pointer("/final_review_state").and_then(Value::as_str),
		Some("rejected")
	);
	assert_eq!(
		contradiction.pointer("/unsupported_claim_flag_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(contradiction.pointer("/source_lineage_count").and_then(Value::as_u64), Some(3));

	let positions = support::array_at(&report, "/reference_positions")?;
	let qmd = support::find_by_field(positions, "/project", "qmd")?;
	let managed = support::find_by_field(positions, "/project", "managed_dreaming_memory_systems")?;
	let always_on =
		support::find_by_field(positions, "/project", "always_on_memory_agent_patterns")?;

	assert_eq!(qmd.pointer("/position").and_then(Value::as_str), Some("untested"));
	assert_eq!(managed.pointer("/position").and_then(Value::as_str), Some("product_reference"));
	assert_eq!(always_on.pointer("/position").and_then(Value::as_str), Some("product_reference"));
	assert!(markdown.contains("ELF now has service-backed live consolidation proposal scoring"));
	assert!(markdown.contains("This is not scheduled production consolidation"));
	assert!(markdown.contains("Source mutations"));
	assert!(markdown.contains("Do not mix knowledge-page rebuild/lint scoring"));
	assert!(
		benchmarking_index.contains("2026-06-16-live-consolidation-proposal-scoring-report.md")
	);
	assert!(readme.contains("Live Consolidation Proposal Scoring Report - June 16, 2026"));
	assert!(readme.contains("real-world-memory-live-consolidation"));
	assert!(benchmark_runbook.contains("Current live consolidation increment"));
	assert!(benchmark_runbook.contains("tmp/real-world-memory/live-consolidation/summary.json"));
	assert!(makefile.contains("[tasks.real-world-memory-live-consolidation]"));
	assert!(makefile.contains("scripts/real-world-docker.sh"));

	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;

	assert_live_consolidation_scripts(&docker_script, &live_script, &live_adapter);

	Ok(())
}

fn assert_live_consolidation_scripts(docker_script: &str, live_script: &str, live_adapter: &str) {
	assert!(docker_script.contains("scripts/real-world-consolidation-live-adapter.sh"));
	assert!(live_script.contains("elf.real_world_consolidation_live_adapter_sweep/v1"));
	assert!(live_script.contains("real_world_live_adapter -- elf"));
	assert!(!live_script.contains("real_world_live_adapter -- qmd"));
	assert!(live_adapter.contains("fn materialize_elf_consolidation("));
	assert!(live_adapter.contains("ConsolidationProposalReviewRequest"));
}

#[test]
fn live_knowledge_page_rebuild_lint_has_dedicated_docker_task() -> Result<()> {
	let workspace = support::workspace_root()?;
	let makefile = fs::read_to_string(workspace.join("Makefile.toml"))?;
	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;
	let live_script =
		fs::read_to_string(workspace.join("scripts/real-world-knowledge-live-adapter.sh"))?;
	let live_adapter = real_world_live_adapter_sources(&workspace)?;
	let knowledge_spec = fs::read_to_string(
		workspace.join("docs").join("spec").join("system_knowledge_pages_v1.md"),
	)?;
	let version_diff_report = fs::read_to_string(
		workspace
			.join("docs")
			.join("evidence")
			.join("benchmarking")
			.join("2026-06-20-knowledge-workspace-version-diff-report.md"),
	)?;
	let benchmark_runbook = fs::read_to_string(
		workspace
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("real_world_agent_memory_benchmark.md"),
	)?;
	let live_runbook = fs::read_to_string(
		workspace
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("live_baseline_benchmark.md"),
	)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert!(makefile.contains("[tasks.real-world-memory-live-knowledge]"));
	assert!(makefile.contains("scripts/real-world-docker.sh"));
	assert!(makefile.contains("memory-live-knowledge"));
	assert!(docker_script.contains("memory-live-knowledge)"));
	assert!(docker_script.contains("-e ELF_KNOWLEDGE_LIVE_REPORT_DIR"));
	assert!(docker_script.contains("-e ELF_KNOWLEDGE_LIVE_FIXTURES"));
	assert!(docker_script.contains("scripts/real-world-knowledge-live-adapter.sh"));
	assert!(live_script.contains("elf.real_world_knowledge_live_adapter_sweep/v1"));
	assert!(live_script.contains("apps/elf-eval/fixtures/real_world_memory/knowledge"));
	assert!(live_script.contains("tmp/real-world-memory/live-knowledge"));
	assert!(live_script.contains("real-world-memory-live-knowledge"));
	assert!(live_script.contains("ElfService knowledge_page_rebuild"));
	assert!(live_script.contains("knowledge_page_lint"));
	assert!(live_script.contains("knowledge_pages_search"));
	assert!(live_script.contains("pages remain derived benchmark artifacts"));
	assert!(live_adapter.contains("\"page_version_diff\""));
	assert!(live_adapter.contains("version_diff_available"));
	assert!(live_adapter.contains("fn materialize_elf_knowledge("));
	assert!(live_adapter.contains("KnowledgePageRebuildRequest"));
	assert!(live_adapter.contains("KnowledgePageLintRequest"));
	assert!(live_adapter.contains("KnowledgePageSearchRequest"));
	assert!(real_world_job_benchmark_sources(&workspace)?.contains("version_diff_coverage"));
	assert!(knowledge_spec.contains("elf.knowledge_page.version_diff/v1"));
	assert!(
		version_diff_report.contains("Knowledge Workspace Version-Diff Report - June 20, 2026")
	);
	assert!(version_diff_report.contains("version_diff_coverage = 1.000"));
	assert!(benchmark_runbook.contains("Current live knowledge-page rebuild/lint increment"));
	assert!(benchmark_runbook.contains("cargo make real-world-memory-live-knowledge"));
	assert!(benchmark_runbook.contains("tmp/real-world-memory/live-knowledge/summary.json"));
	assert!(live_runbook.contains("cargo make real-world-memory-live-knowledge"));
	assert!(benchmarking_index.contains("2026-06-20-live-knowledge-page-rebuild-lint-report.md"));
	assert!(benchmarking_index.contains("2026-06-20-knowledge-workspace-version-diff-report.md"));
	assert!(readme.contains("Live Knowledge-Page Rebuild/Lint Report - June 20, 2026"));
	assert!(readme.contains("Knowledge Workspace Version-Diff Report - June 20, 2026"));

	Ok(())
}

#[test]
fn runner_discovers_nested_fixture_layout() -> Result<()> {
	let report = support::run_json_report_from(support::fixture_root())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(82));

	Ok(())
}

#[test]
fn operator_debug_fixture_reports_trace_links_and_failure_details() -> Result<()> {
	let report = support::run_json_report_from(support::operator_debug_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(7));
	assert_eq!(
		report.pointer("/summary/operator_debug_job_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(report.pointer("/summary/raw_sql_needed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/trace_incomplete_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/operator_ux_gap_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(7));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(3)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let dropped = support::find_by_field(jobs, "/job_id", "operator-debug-dropped-evidence-001")?;
	let selected =
		support::find_by_field(jobs, "/job_id", "operator-debug-selected-not-narrated-001")?;
	let compact =
		support::find_by_field(jobs, "/job_id", "operator-debug-qmd-style-compact-replay-001")?;

	assert_eq!(dropped.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		dropped.pointer("/operator_debug/raw_sql_needed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		dropped.pointer("/operator_debug/dropped_candidate_visibility").and_then(Value::as_str),
		Some("visible in Retrieval Funnel and Replay Candidates")
	);
	assert_eq!(
		dropped.pointer("/operator_debug/viewer_url").and_then(Value::as_str),
		Some("/viewer?trace_id=11111111-1111-4111-8111-111111111111")
	);
	assert_eq!(
		dropped.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("filter.read_profile")
	);
	assert!(support::array_contains_str(
		dropped,
		"/trace_explainability/stages/1/dropped_evidence",
		"trace-dropped-expected"
	)?);
	assert!(support::array_contains_str(
		dropped,
		"/trace_explainability/stages/1/distractor_evidence",
		"trace-dropped-decoy"
	)?);
	assert!(support::array_contains_str(dropped, "/produced_evidence", "trace-dropped-expected")?);
	assert_eq!(selected.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		selected.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("selection.narration")
	);
	assert_eq!(
		selected.pointer("/operator_debug/failure_mode").and_then(Value::as_str),
		Some("selected_but_not_narrated")
	);
	assert_eq!(compact.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		compact.pointer("/operator_debug/failure_mode").and_then(Value::as_str),
		Some("qmd_style_compact_replay")
	);
	assert_eq!(
		compact.pointer("/operator_debug/replay_command_available").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		compact.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("recall_debug.compact_replay")
	);
	assert!(support::array_contains_str(
		compact,
		"/trace_explainability/stages/4/kept_evidence",
		"compact-replay-artifact"
	)?);
	assert!(support::array_contains_str(
		compact,
		"/produced_evidence",
		"qmd-short-replay-reference"
	)?);

	Ok(())
}

#[test]
fn consolidation_fixtures_report_reviewable_proposal_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::consolidation_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(
		report.pointer("/summary/consolidation/proposal_count").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/consolidation/proposal_unsupported_claim_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/executable_gap_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/lineage_completeness").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/consolidation/review_action_correctness").and_then(Value::as_f64),
		Some(1.0)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let project_summary =
		support::find_by_field(jobs, "/job_id", "consolidation-project-summary-apply-001")?;
	let contradiction =
		support::find_by_field(jobs, "/job_id", "consolidation-contradiction-report-discard-001")?;

	assert_eq!(
		project_summary
			.pointer("/consolidation/proposals/0/actual_review_action")
			.and_then(Value::as_str),
		Some("apply")
	);
	assert_eq!(
		contradiction
			.pointer("/consolidation/proposals/0/actual_review_action")
			.and_then(Value::as_str),
		Some("discard")
	);
	assert_eq!(
		contradiction
			.pointer("/consolidation/proposals/0/unsupported_claim_count")
			.and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(&report, "/suites")?;
	let consolidation_suite = support::find_by_field(suites, "/suite_id", "consolidation")?;

	assert_eq!(consolidation_suite.pointer("/status").and_then(Value::as_str), Some("pass"));

	Ok(())
}

#[test]
fn knowledge_fixtures_report_page_metrics() -> Result<()> {
	let report = support::run_json_report_from(support::knowledge_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/knowledge/page_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		report.pointer("/summary/knowledge/section_count").and_then(Value::as_u64),
		Some(13)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/citation_coverage").and_then(Value::as_f64),
		Some(0.923)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/rebuild_determinism").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_count").and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/pages_with_backlinks").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/backlink_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/page_usefulness").and_then(Value::as_f64),
		Some(0.979)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/pages_with_version_diff").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/allowed_variance_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = support::array_at(&report, "/suites")?;
	let knowledge_suite = support::find_by_field(suites, "/suite_id", "knowledge_compilation")?;

	assert_eq!(knowledge_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(knowledge_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = support::array_at(&report, "/jobs")?;
	let project_page_job = support::find_by_field(jobs, "/job_id", "knowledge-project-page-001")?;
	let watch_rebuild_job = support::find_by_field(jobs, "/job_id", "knowledge-watch-rebuild-003")?;

	assert_eq!(
		project_page_job.pointer("/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		project_page_job.pointer("/knowledge/untraced_section_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		watch_rebuild_job.pointer("/knowledge/pages_with_version_diff").and_then(Value::as_u64),
		Some(1)
	);
	assert!(
		watch_rebuild_job
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| answer
				.contains("PageIndex/OpenKB adapter claim as lint evidence")
				&& answer.contains("leaves source documents plus Memory Notes unmodified"))
	);

	Ok(())
}

#[test]
fn project_decisions_fixtures_report_decision_policy_cases() -> Result<()> {
	let report = support::run_json_report_from(support::project_decisions_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);

	let suites = support::array_at(&report, "/suites")?;
	let project_decisions = support::find_by_field(suites, "/suite_id", "project_decisions")?;

	assert_eq!(project_decisions.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(project_decisions.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(
		project_decisions.pointer("/update_rationale_available_count").and_then(Value::as_u64),
		Some(5)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let accepted =
		support::find_by_field(jobs, "/job_id", "project-decision-accepted-typed-failures-001")?;
	let reversal =
		support::find_by_field(jobs, "/job_id", "project-decision-reversal-live-baseline-001")?;
	let validation =
		support::find_by_field(jobs, "/job_id", "project-decision-current-validation-gate-001")?;
	let tradeoff =
		support::find_by_field(jobs, "/job_id", "project-decision-tradeoff-fixture-backed-001")?;
	let caveat =
		support::find_by_field(jobs, "/job_id", "project-decision-private-manifest-caveat-001")?;

	assert_eq!(accepted.pointer("/answer_type").and_then(Value::as_str), Some("decision_record"));
	assert_eq!(
		accepted.pointer("/expected_evidence").and_then(Value::as_array).map(Vec::len),
		Some(2)
	);
	assert_eq!(
		reversal.pointer("/evolution/historical_evidence/0").and_then(Value::as_str),
		Some("live-baseline-suite-win-old")
	);
	assert_eq!(
		validation.pointer("/evolution/current_evidence/0").and_then(Value::as_str),
		Some("validation-gate-current-decodex")
	);
	assert_eq!(tradeoff.pointer("/requires_caveat").and_then(Value::as_bool), Some(true));
	assert_eq!(caveat.pointer("/can_answer_unknown").and_then(Value::as_bool), Some(true));

	for job in jobs {
		let expected_evidence = support::array_at(job, "/expected_evidence")?;

		assert!(
			!expected_evidence.is_empty(),
			"project decision job {} must declare required evidence",
			job.pointer("/job_id").and_then(Value::as_str).unwrap_or("<unknown>")
		);
	}
	for entry in fs::read_dir(support::project_decisions_fixture_dir())? {
		let path = entry?.path();

		if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
			continue;
		}

		let fixture = serde_json::from_str::<Value>(&fs::read_to_string(path)?)?;
		let required_evidence = support::array_at(&fixture, "/required_evidence")?;
		let negative_traps = support::array_at(&fixture, "/negative_traps")?;

		assert!(!required_evidence.is_empty());
		assert!(!negative_traps.is_empty());
	}

	Ok(())
}
