use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::{consolidation_knowledge::consolidation_knowledge_tests_helpers, support};

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
	let live_adapter =
		consolidation_knowledge_tests_helpers::real_world_live_adapter_sources(&workspace)?;

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
	let live_adapter =
		consolidation_knowledge_tests_helpers::real_world_live_adapter_sources(&workspace)?;
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
	assert!(
		consolidation_knowledge_tests_helpers::real_world_job_benchmark_sources(&workspace)?
			.contains("version_diff_coverage")
	);
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
