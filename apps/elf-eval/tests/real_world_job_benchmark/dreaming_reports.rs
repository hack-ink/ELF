use std::{fs, path::Path};

use color_eyre::{self, eyre};
use serde_json::Value;

use crate::support;

fn read_rust_module_sources(src_dir: &Path, module_name: &str) -> color_eyre::Result<String> {
	let module_root = src_dir.join(format!("{module_name}.rs"));
	let module_dir = src_dir.join(module_name);
	let mut source = fs::read_to_string(module_root)?;

	if module_dir.is_dir() {
		let mut entries = fs::read_dir(module_dir)?
			.map(|entry| entry.map(|entry| entry.path()))
			.collect::<std::io::Result<Vec<_>>>()?;

		entries.retain(|path| path.extension().is_some_and(|extension| extension == "rs"));
		entries.sort();

		for path in entries {
			source.push('\n');
			source.push_str(&fs::read_to_string(path)?);
		}
	}

	Ok(source)
}

#[test]
fn live_temporal_reconciliation_report_records_xy905_before_after() -> color_eyre::Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::live_temporal_reconciliation_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::live_temporal_reconciliation_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.live_temporal_reconciliation_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-905"));
	assert_eq!(
		report
			.pointer("/baseline/elf_memory_evolution/job_status_counts/pass")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/baseline/elf_memory_evolution/job_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/post_stage/elf_memory_evolution/job_status_counts/pass")
			.and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/post_stage/elf_memory_evolution/job_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/post_stage/elf_memory_evolution/suite_status").and_then(Value::as_str),
		Some("pass")
	);
	assert_eq!(
		report.pointer("/post_stage/qmd_memory_evolution/suite_status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		report
			.pointer("/comparison_judgment/current_vs_historical_correctness")
			.and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(
		report
			.pointer("/comparison_judgment/deletion_ttl_tombstone_behavior")
			.and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(support::array_contains_str(
		&report,
		"/trace_contract/answer_fields",
		"selected_historical_evidence"
	)?);
	assert!(support::array_contains_str(
		&report,
		"/trace_contract/materialization_fields",
		"current_winner_evidence_ids"
	)?);
	assert!(support::array_contains_str(
		&report,
		"/trace_contract/trace_stages",
		"temporal_reconciliation.conflict_candidates"
	)?);
	assert!(report.pointer("/trace_contract/negative_gate").and_then(Value::as_str).is_some_and(
		|gate| gate.contains("selected conflict evidence id") && gate.contains("wrong_result")
	));
	assert!(markdown.contains("ELF passing all six memory-evolution jobs"));
	assert!(markdown.contains("selected-but-not-narrated conflicts as `wrong_result`"));
	assert!(markdown.contains("Do not claim ELF beats Graphiti/Zep"));
	assert!(benchmarking_index.contains("2026-06-16-live-temporal-reconciliation-report.md"));
	assert!(
		readme.contains("Live Temporal Reconciliation Report - June 16, 2026")
			&& readme.contains("now reports ELF live `memory_evolution` as 6/6 pass")
	);

	Ok(())
}

#[test]
fn dreaming_competitor_strength_retest_report_closes_xy955_without_overclaims()
-> color_eyre::Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::dreaming_competitor_strength_retest_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::dreaming_competitor_strength_retest_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_competitor_strength_retest_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-955"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("locally_and_partially_stronger_only")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/regressed_stage_count").and_then(Value::as_u64), Some(0));
	assert!(support::array_contains_str(&report, "/status_terms", "typed_non_pass")?);
	assert!(support::array_contains_str(
		&report,
		"/summary/unsupported_claims_rejected",
		"ELF does not broadly beat qmd from this retest."
	)?);

	assert_xy955_commands(&report)?;
	assert_xy955_stage_closeout(&report)?;
	assert_xy955_scenario_retests(&report)?;
	assert_xy955_optimization_queue(&report)?;
	assert_xy955_follow_up_issue_briefs(&report)?;

	assert!(markdown.contains("ELF is locally and partially stronger"));
	assert!(
		markdown.contains("The full live-adapter command now has fresh ELF and qmd scored reports")
	);
	assert!(
		markdown.contains(
			"Do not treat qmd full-suite wrong_result counts as a regression of qmd debug"
		)
	);
	assert!(markdown.contains("## Follow-Up Issue Briefs"));
	assert!(markdown.contains(
		"| GraphRAG/LightRAG/RAGFlow/llm-wiki/gbrain/graphify citation/navigation/knowledge surfaces |"
	));
	assert!(
		benchmarking_index.contains("2026-06-17-dreaming-competitor-strength-retest-report.md")
	);
	assert!(readme.contains("Dreaming Competitor-Strength Retest Report - June 17, 2026"));
	assert!(readme.contains("17 competitor-strength closeout"));

	Ok(())
}

#[test]
fn qmd_debug_ergonomics_dreaming_retest_report_preserves_qmd_edge() -> color_eyre::Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::qmd_debug_ergonomics_dreaming_retest_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::qmd_debug_ergonomics_dreaming_retest_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_qmd_debug_retest_summary(&report)?;
	assert_qmd_debug_retest_command_and_adapters(&report)?;
	assert_qmd_debug_retest_scenarios(&report)?;
	assert_qmd_debug_retest_boundaries(&report)?;
	assert_qmd_debug_retest_markdown_and_indexes(&markdown, &benchmarking_index, &readme);

	Ok(())
}

fn assert_qmd_debug_retest_summary(report: &Value) -> color_eyre::Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.qmd_debug_ergonomics_dreaming_retest_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-982"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("unchanged_with_live_operator_debug_confirmation")
	);
	assert_eq!(
		report.pointer("/summary/debug_ergonomics_edge").and_then(Value::as_str),
		Some("qmd_default_top10_and_short_cli_replay_preserved")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/improved_scenario_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/regressed_scenario_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/unchanged_scenario_count").and_then(Value::as_u64),
		Some(6)
	);
	assert!(support::array_contains_str(
		report,
		"/summary/unsupported_claims_rejected",
		"qmd's live operator-debug wrong_result rows do not erase qmd's default top-k and short CLI replay edge."
	)?);

	Ok(())
}

fn assert_qmd_debug_retest_command_and_adapters(report: &Value) -> color_eyre::Result<()> {
	let command = support::find_by_field(
		support::array_at(report, "/commands")?,
		"/command",
		"cargo make real-world-job-operator-ux-live-adapters",
	)?;

	assert_eq!(command.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		command.pointer("/summary/schema").and_then(Value::as_str),
		Some("elf.real_world_operator_debug_live_adapter_sweep/v1")
	);

	let adapters = support::array_at(report, "/adapter_summaries")?;
	let elf = support::find_by_field(adapters, "/adapter_id", "elf_operator_debug_live")?;
	let qmd = support::find_by_field(adapters, "/adapter_id", "qmd_operator_debug_live")?;

	assert_eq!(elf.pointer("/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(elf.pointer("/trace_available_count").and_then(Value::as_u64), Some(6));
	assert_eq!(elf.pointer("/replay_command_available_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(qmd.pointer("/wrong_result").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/trace_available_count").and_then(Value::as_u64), Some(0));
	assert_eq!(qmd.pointer("/trace_incomplete_count").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd.pointer("/replay_command_available_count").and_then(Value::as_u64), Some(6));

	Ok(())
}

fn assert_qmd_debug_retest_scenarios(report: &Value) -> color_eyre::Result<()> {
	let scenarios = support::array_at(report, "/scenario_retests")?;
	let top10 =
		support::find_by_field(scenarios, "/scenario_id", "qmd_default_top10_candidate_artifact")?;
	let replay = support::find_by_field(scenarios, "/scenario_id", "qmd_short_cli_replay")?;
	let trace =
		support::find_by_field(scenarios, "/scenario_id", "elf_operator_debug_trace_hydration")?;
	let candidate = support::find_by_field(
		scenarios,
		"/scenario_id",
		"operator_debug_candidate_drop_visibility",
	)?;
	let expansion =
		support::find_by_field(scenarios, "/scenario_id", "query_expansion_attribution")?;
	let fusion = support::find_by_field(scenarios, "/scenario_id", "fusion_attribution")?;
	let rerank = support::find_by_field(scenarios, "/scenario_id", "rerank_attribution")?;

	assert_eq!(scenarios.len(), 10);
	assert_eq!(top10.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(top10.pointer("/current_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(replay.pointer("/current_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(
		trace.pointer("/current_counts/elf_trace_available").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		trace.pointer("/current_counts/qmd_trace_available").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		candidate
			.pointer("/current_counts/qmd_intermediate_stage_visible_jobs")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert!(support::array_contains_str(
		candidate,
		"/typed_non_pass_states",
		"retrieved_but_dropped"
	)?);
	assert_eq!(expansion.pointer("/judgment").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(fusion.pointer("/judgment").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(rerank.pointer("/judgment").and_then(Value::as_str), Some("non_goal"));

	Ok(())
}

fn assert_qmd_debug_retest_boundaries(report: &Value) -> color_eyre::Result<()> {
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/allowed",
		"qmd's default local-debug edge remains: top-10 candidate rows plus short CLI replay."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF broadly beats qmd from this retest."
	)?);
	assert!(support::array_contains_str(
		report,
		"/next_optimization_direction/required_fields",
		"fusion_rank_deltas"
	)?);

	Ok(())
}

fn assert_qmd_debug_retest_markdown_and_indexes(
	markdown: &str,
	benchmarking_index: &str,
	readme: &str,
) {
	assert!(markdown.contains("The qmd debug-ergonomics outcome is unchanged"));
	assert!(markdown.contains("ELF 6 pass/0 wrong_result; qmd 0 pass/6 wrong_result"));
	assert!(
		markdown.contains("Do not treat qmd's 0 pass/6 wrong_result live operator-debug slice")
	);
	assert!(markdown.contains("Immediate top-k rows with source id"));
	assert!(
		benchmarking_index.contains("2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md")
	);
	assert!(readme.contains("qmd Debug-Ergonomics Dreaming Retest Report - June 19, 2026"));
	assert!(readme.contains("Temporal and Trajectory Adapter Coverage Report - June 23, 2026"));
	assert!(readme.contains("Latest real-world benchmark report: June 27, 2026"));
	assert!(readme.contains("keeps the qmd edge unchanged"));
}

#[test]
fn openviking_trajectory_materialization_report_preserves_blocked_gates() -> color_eyre::Result<()>
{
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::openviking_trajectory_materialization_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::openviking_trajectory_materialization_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_openviking_trajectory_materialization_summary(&report)?;
	assert_openviking_trajectory_materialization_command(&report)?;
	assert_openviking_trajectory_materialization_scenarios(&report)?;
	assert_openviking_trajectory_materialization_boundaries(&report)?;
	assert_openviking_trajectory_materialization_markdown_and_indexes(
		&markdown,
		&benchmarking_index,
		&readme,
	);

	Ok(())
}

#[test]
fn letta_core_archive_export_readback_report_preserves_blocked_gates() -> color_eyre::Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::letta_core_archive_export_readback_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::letta_core_archive_export_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.letta_core_archive_export_readback_summary/v1")
	);
	assert_eq!(
		report.pointer("/adapter_id").and_then(Value::as_str),
		Some("letta_core_archive_export_readback")
	);
	assert_eq!(
		report.pointer("/materialization/status/failure_class").and_then(Value::as_str),
		Some("letta_live_run_disabled")
	);
	assert_eq!(
		report.pointer("/materialization/status/overall").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/status").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/counts/blocked").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/materialization/scored_benchmark/counts/pass").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/scored_benchmark/counts/wrong_result")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/scored_benchmark/evidence_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report
			.pointer("/materialization/benchmark_input/core_blocks")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(9)
	);
	assert_eq!(
		report
			.pointer("/materialization/benchmark_input/archival_passages")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(6)
	);
	assert_eq!(
		report
			.pointer("/materialization/evidence_mapping/expected_evidence_ids")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(14)
	);
	assert_eq!(
		report
			.pointer("/materialization/evidence_mapping/mapped_evidence_ids")
			.and_then(Value::as_array)
			.map(Vec::len),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/materialization/improvement_regression_readback/judgment")
			.and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(support::array_contains_str(
		&report,
		"/materialization/claim_boundaries/not_allowed",
		"Do not claim ELF beats Letta on core-vs-archival memory from fixture-only ELF evidence."
	)?);
	assert!(markdown.contains("The Letta follow-up is now reproducible"));
	assert!(markdown.contains("6 typed blocked"));
	assert!(markdown.contains("competitive status is unchanged"));
	assert!(benchmarking_index.contains("2026-06-19-letta-core-archive-export-readback-report.md"));
	assert!(readme.contains("Letta core/archive materialization after XY-984"));
	assert!(readme.contains("smoke-letta-core-archive-export-readback"));

	Ok(())
}

#[test]
fn service_native_dreaming_readback_report_materializes_public_jobs() -> color_eyre::Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::service_native_dreaming_readback_report_json_path()?,
	)?)?;
	let materialization = serde_json::from_str::<Value>(&fs::read_to_string(
		support::service_native_dreaming_readback_materialization_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::service_native_dreaming_readback_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_service_native_dreaming_report_summary(&report)?;
	assert_service_native_dreaming_report_jobs(&report)?;
	assert_service_native_dreaming_materialization(&materialization)?;
	assert_service_native_dreaming_docs(&markdown, &benchmarking_index, &readme);

	Ok(())
}

fn assert_service_native_dreaming_report_summary(report: &Value) -> color_eyre::Result<()> {
	assert_eq!(
		report.pointer("/adapter/adapter_id").and_then(Value::as_str),
		Some("elf_service_native_dreaming")
	);
	assert_eq!(
		report.pointer("/adapter/behavior").and_then(Value::as_str),
		Some("service_native_dreaming_readback")
	);
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(11));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(9));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/proactive_brief/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/scheduled_memory/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	let suites = support::array_at(report, "/suites")?;
	let memory = support::find_by_field(suites, "/suite_id", "memory_summary")?;
	let proactive = support::find_by_field(suites, "/suite_id", "proactive_brief")?;
	let scheduled = support::find_by_field(suites, "/suite_id", "scheduled_memory")?;

	assert_eq!(memory.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(proactive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));

	Ok(())
}

fn assert_service_native_dreaming_report_jobs(report: &Value) -> color_eyre::Result<()> {
	let jobs = support::array_at(report, "/jobs")?;
	let memory = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;
	let daily = support::find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private_brief =
		support::find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;
	let weekly =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;
	let private_scheduled = support::find_by_field(
		jobs,
		"/job_id",
		"scheduled-private-provider-scheduler-blocked-001",
	)?;

	assert_eq!(memory.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(daily.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(private_brief.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(private_scheduled.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(!support::array_contains_str(memory, "/produced_evidence", "stale-summary-gap")?);
	assert!(!support::array_contains_str(memory, "/produced_evidence", "summary-temporary-claim")?);
	assert!(!support::array_contains_str(daily, "/produced_evidence", "daily-old-parity-trap")?);
	assert!(!support::array_contains_str(
		weekly,
		"/produced_evidence",
		"scheduled-weekly-hosted-parity-trap"
	)?);

	Ok(())
}

fn assert_service_native_dreaming_materialization(
	materialization: &Value,
) -> color_eyre::Result<()> {
	assert_eq!(
		materialization.pointer("/schema").and_then(Value::as_str),
		Some("elf.real_world_live_adapter_materialization/v1")
	);
	assert_eq!(
		materialization.pointer("/adapter_id").and_then(Value::as_str),
		Some("elf_service_native_dreaming")
	);
	assert_eq!(materialization.pointer("/status").and_then(Value::as_str), Some("blocked"));

	let jobs = support::array_at(materialization, "/jobs")?;
	let memory = support::find_by_field(jobs, "/job_id", "memory-summary-source-trace-001")?;
	let daily = support::find_by_field(jobs, "/job_id", "proactive-daily-project-brief-001")?;
	let private_brief =
		support::find_by_field(jobs, "/job_id", "proactive-private-corpus-refresh-blocked-001")?;

	for job in jobs {
		match job.pointer("/status").and_then(Value::as_str) {
			Some("pass") => {
				assert_eq!(
					job.pointer("/dreaming_readback/runtime_path").and_then(Value::as_str),
					Some("ElfService::add_note -> ElfService::list -> derived readback artifact")
				);
				assert!(
					support::array_at(job, "/dreaming_readback/missing_source_refs")?.is_empty()
				);
				assert_eq!(
					job.pointer("/dreaming_readback/source_mutation_count").and_then(Value::as_u64),
					Some(0)
				);
				assert_eq!(
					job.pointer("/dreaming_readback/no_source_mutation_checked")
						.and_then(Value::as_bool),
					Some(true)
				);
			},
			Some("blocked") => {
				assert!(job.pointer("/dreaming_readback").is_none_or(Value::is_null));
			},
			status => {
				return Err(eyre::eyre!(
					"unexpected service-native materialization status: {status:?}"
				));
			},
		}
	}

	assert!(support::array_contains_str(
		memory,
		"/dreaming_readback/selected_source_refs",
		"stale-summary-gap"
	)?);
	assert!(!support::array_contains_str(memory, "/evidence_ids", "stale-summary-gap")?);
	assert!(support::array_contains_str(
		daily,
		"/dreaming_readback/selected_source_refs",
		"daily-old-parity-trap"
	)?);
	assert!(!support::array_contains_str(daily, "/evidence_ids", "daily-old-parity-trap")?);
	assert!(private_brief.pointer("/dreaming_readback").is_none_or(Value::is_null));

	Ok(())
}

fn assert_service_native_dreaming_docs(markdown: &str, benchmarking_index: &str, readme: &str) {
	assert!(markdown.contains("9 pass"));
	assert!(markdown.contains("0 wrong_result"));
	assert!(markdown.contains("2 typed blocked"));
	assert!(markdown.contains("ElfService::add_note -> ElfService::list"));
	assert!(markdown.contains("Do not claim ELF broadly beats OpenAI Pulse"));
	assert!(benchmarking_index.contains("2026-06-19-service-native-dreaming-readback-report.md"));
	assert!(readme.contains("Service-native Dreaming readback after XY-986"));
	assert!(readme.contains("real-world-memory-service-native-dreaming"));
}

#[test]
fn dreaming_review_queue_report_wires_reviewable_policy_contract() -> color_eyre::Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::dreaming_review_queue_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::dreaming_review_queue_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let workspace = support::workspace_root()?;
	let service = read_rust_module_sources(
		&workspace.join("packages/elf-service/src"),
		"dreaming_review_queue",
	)?;
	let service_lib = fs::read_to_string(workspace.join("packages/elf-service/src/lib.rs"))?;
	let routes = read_rust_module_sources(&workspace.join("apps/elf-api/src"), "routes")?;
	let mcp = fs::read_to_string(workspace.join("apps/elf-mcp/src/server.rs"))?;
	let consolidation_spec =
		fs::read_to_string(workspace.join("docs/spec/system_consolidation_proposals_v1.md"))?;
	let service_spec =
		fs::read_to_string(workspace.join("docs/spec/system_elf_memory_service_v2.md"))?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_review_queue_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1021"));
	assert_eq!(
		report.pointer("/summary/queue_schema").and_then(Value::as_str),
		Some("elf.dreaming_review_queue/v1")
	);
	assert_eq!(
		report.pointer("/summary/source_mutation_allowed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report.pointer("/summary/high_impact_requires_review").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(report.pointer("/summary/variant_count").and_then(Value::as_u64), Some(9));

	for suite in ["memory_summary", "proactive_brief", "scheduled_memory", "consolidation"] {
		assert!(support::array_contains_str(&report, "/summary/covered_existing_suites", suite)?);
	}
	for variant in
		["tag", "duplicate_merge", "page_rebuild", "memory_promotion", "graph_fact", "correction"]
	{
		assert!(support::array_contains_str(&report, "/summary/covered_future_variants", variant)?);

		support::find_by_field(
			support::array_at(&report, "/queue_variants")?,
			"/variant",
			variant,
		)?;
	}
	for field in [
		"source_refs",
		"affected_refs",
		"confidence",
		"unsupported_claim_flags",
		"diff",
		"policy",
		"review_audit",
	] {
		assert!(support::array_contains_str(&report, "/required_item_fields", field)?);
	}

	assert!(service.contains("ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1"));
	assert!(service.contains("pub async fn dreaming_review_queue"));
	assert!(service.contains("source_mutation_allowed: false"));
	assert!(service.contains("low_risk_derived_organization"));
	assert!(service.contains("available_review_actions"));
	assert!(service_lib.contains("pub mod dreaming_review_queue"));
	assert!(service_lib.contains("DreamingReviewQueueResponse"));
	assert!(routes.contains("/v2/admin/dreaming/review-queue"));
	assert!(routes.contains("DreamingReviewQueueRequest"));
	assert!(routes.contains("async fn dreaming_review_queue"));
	assert!(mcp.contains("elf_dreaming_review_queue"));
	assert!(mcp.contains("dreaming_review_queue_schema"));
	assert!(mcp.contains("/v2/admin/dreaming/review-queue"));
	assert!(consolidation_spec.contains("elf.dreaming_review_queue/v1"));
	assert!(consolidation_spec.contains("source_mutation_allowed"));
	assert!(consolidation_spec.contains("duplicate_merge"));
	assert!(service_spec.contains("GET /v2/admin/dreaming/review-queue"));
	assert!(service_spec.contains("source refs, affected refs, confidence"));
	assert!(markdown.contains("Dreaming Review Queue Report"));
	assert!(markdown.contains("Auto-apply is limited to approved low-risk"));
	assert!(benchmarking_index.contains("2026-06-20-dreaming-review-queue-report.md"));
	assert!(readme.contains("Dreaming review queue after XY-1021"));
	assert!(readme.contains("elf.dreaming_review_queue/v1"));

	Ok(())
}

fn assert_openviking_trajectory_materialization_summary(report: &Value) -> color_eyre::Result<()> {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.openviking_trajectory_materialization_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-983"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("materialized_blocked_context_trajectory_evidence")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/blockers_removed_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked_scenario_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/regressed_scenario_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert!(support::array_contains_str(
		report,
		"/summary/unsupported_claims_rejected",
		"ELF does not beat OpenViking staged retrieval trajectory from fixture-only blocked rows."
	)?);

	Ok(())
}

fn assert_openviking_trajectory_materialization_command(report: &Value) -> color_eyre::Result<()> {
	let command = support::find_by_field(
		support::array_at(report, "/commands")?,
		"/command",
		"cargo make real-world-memory-context-trajectory",
	)?;
	let summary =
		command.pointer("/summary").ok_or_else(|| eyre::eyre!("missing command summary"))?;

	assert_eq!(command.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		command.pointer("/artifact_json").and_then(Value::as_str),
		Some("tmp/real-world-memory/context-trajectory/report.json")
	);
	assert_eq!(summary.pointer("/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(summary.pointer("/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(summary.pointer("/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(summary.pointer("/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(summary.pointer("/evidence_covered_count").and_then(Value::as_u64), Some(9));
	assert_eq!(summary.pointer("/source_ref_covered_count").and_then(Value::as_u64), Some(9));
	assert_eq!(summary.pointer("/quote_covered_count").and_then(Value::as_u64), Some(9));

	Ok(())
}

fn assert_openviking_trajectory_materialization_scenarios(
	report: &Value,
) -> color_eyre::Result<()> {
	let scenarios = support::array_at(report, "/scenario_materialization")?;
	let staged = support::find_by_field(
		scenarios,
		"/scenario_id",
		"openviking_staged_retrieval_trajectory",
	)?;
	let hierarchy =
		support::find_by_field(scenarios, "/scenario_id", "openviking_hierarchy_selection")?;
	let recursive = support::find_by_field(
		scenarios,
		"/scenario_id",
		"openviking_recursive_context_expansion",
	)?;

	assert_eq!(scenarios.len(), 3);

	for scenario in [staged, hierarchy, recursive] {
		assert_eq!(scenario.pointer("/previous_status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(scenario.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	}

	assert!(support::array_contains_str(
		staged,
		"/produced_evidence",
		"openviking-evidence-id-output-contract"
	)?);
	assert!(support::array_contains_str(
		hierarchy,
		"/produced_evidence",
		"hierarchy-selection-output-contract"
	)?);
	assert!(support::array_contains_str(
		recursive,
		"/produced_evidence",
		"recursive-expansion-output-contract"
	)?);
	assert_eq!(
		staged.pointer("/claim_boundary").and_then(Value::as_str),
		Some(
			"No ELF win, tie, or loss is allowed until both systems publish comparable stage artifacts for the same context-trajectory scenario."
		)
	);
	assert_eq!(
		hierarchy.pointer("/blocker").and_then(Value::as_str),
		Some("selected_hierarchy_nodes_and_evidence_ids_missing")
	);
	assert_eq!(
		recursive.pointer("/blocker").and_then(Value::as_str),
		Some("expansion_paths_and_same_corpus_evidence_ids_missing")
	);

	Ok(())
}

fn assert_openviking_trajectory_materialization_boundaries(
	report: &Value,
) -> color_eyre::Result<()> {
	assert_eq!(
		report.pointer("/improvement_regression_readback/improved").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/improvement_regression_readback/blocked").and_then(Value::as_u64),
		Some(3)
	);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/allowed",
		"The context-trajectory slice is now reproducible through cargo make real-world-memory-context-trajectory."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF beats OpenViking on staged retrieval trajectory."
	)?);
	assert!(support::array_contains_str(
		report,
		"/next_optimization_direction/required_fields",
		"expansion_path"
	)?);
	assert_eq!(
		report.pointer("/next_optimization_direction/non_goal").and_then(Value::as_str),
		Some(
			"No ELF product change or superiority claim is authorized by this materialization-only report."
		)
	);

	Ok(())
}

fn assert_openviking_trajectory_materialization_markdown_and_indexes(
	markdown: &str,
	benchmarking_index: &str,
	readme: &str,
) {
	assert!(markdown.contains("The OpenViking trajectory follow-up is now materialized"));
	assert!(markdown.contains("3 encoded jobs, 0 pass, 3 blocked, 9/9 evidence coverage"));
	assert!(markdown.contains("Do not claim ELF beats OpenViking on staged retrieval trajectory."));
	assert!(markdown.contains("OpenViking context-trajectory job can move from `blocked`"));
	assert!(
		benchmarking_index.contains("2026-06-19-openviking-trajectory-materialization-report.md")
	);
	assert!(readme.contains("OpenViking Trajectory Materialization Report - June 19, 2026"));
	assert!(readme.contains("cargo make real-world-memory-context-trajectory"));
	assert!(readme.contains("3 typed blockers with 9/9 evidence coverage"));
}

fn assert_xy955_commands(report: &Value) -> color_eyre::Result<()> {
	let commands = support::array_at(report, "/commands")?;
	let aggregate = support::find_by_field(commands, "/command", "cargo make real-world-memory")?;
	let graph_rag =
		support::find_by_field(commands, "/command", "cargo make real-world-memory-graph-rag")?;
	let first_generation =
		support::find_by_field(commands, "/command", "cargo make real-world-first-generation-oss")?;
	let live =
		support::find_by_field(commands, "/command", "cargo make real-world-memory-live-adapters")?;

	assert_eq!(aggregate.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(aggregate.pointer("/summary/pass").and_then(Value::as_u64), Some(53));
	assert_eq!(aggregate.pointer("/summary/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(graph_rag.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(graph_rag.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(graph_rag.pointer("/summary/incomplete").and_then(Value::as_u64), Some(1));
	assert_eq!(graph_rag.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(first_generation.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(first_generation.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(live.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		live.pointer("/partial_summary/elf_live_real_world/pass").and_then(Value::as_u64),
		Some(40)
	);
	assert_eq!(
		live.pointer("/partial_summary/elf_live_real_world/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		live.pointer("/partial_summary/qmd_live_real_world/pass").and_then(Value::as_u64),
		Some(17)
	);
	assert_eq!(
		live.pointer("/partial_summary/qmd_live_real_world/wrong_result").and_then(Value::as_u64),
		Some(13)
	);

	Ok(())
}

fn assert_xy955_stage_closeout(report: &Value) -> color_eyre::Result<()> {
	let stages = support::array_at(report, "/stage_closeout")?;

	assert_eq!(stages.len(), 8);

	let current = support::find_by_field(stages, "/stage_id", "current_vs_historical_correctness")?;
	let proactive = support::find_by_field(stages, "/stage_id", "proactive_brief_readiness")?;
	let scheduled = support::find_by_field(stages, "/stage_id", "scheduled_memory_task_readiness")?;
	let final_retest =
		support::find_by_field(stages, "/stage_id", "final_competitor_retest_status")?;

	assert_eq!(current.pointer("/judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(current.pointer("/current_counts/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(current.pointer("/current_counts/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(proactive.pointer("/judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(proactive.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(scheduled.pointer("/current_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(scheduled.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(final_retest.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(final_retest.pointer("/current_counts/pass").and_then(Value::as_u64), Some(40));
	assert_eq!(
		final_retest.pointer("/current_counts/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(final_retest.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(
		final_retest.pointer("/current_counts/not_encoded").and_then(Value::as_u64),
		Some(19)
	);
	assert!(final_retest.pointer("/boundary").and_then(Value::as_str).is_some_and(|boundary| {
		boundary.contains("qmd now has a fresh scored live report")
			&& boundary.contains("broader superiority is not proven")
	}));
	assert_eq!(final_retest.pointer("/qmd_current_counts/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(
		final_retest.pointer("/qmd_current_counts/wrong_result").and_then(Value::as_u64),
		Some(13)
	);

	Ok(())
}

fn assert_xy955_scenario_retests(report: &Value) -> color_eyre::Result<()> {
	let scenarios = support::array_at(report, "/scenario_retests")?;
	let qmd = support::find_by_field(scenarios, "/scenario_id", "qmd_debug_ergonomics")?;
	let mem0 = support::find_by_field(
		scenarios,
		"/scenario_id",
		"mem0_openmemory_preference_history_export",
	)?;
	let letta = support::find_by_field(scenarios, "/scenario_id", "letta_core_archive")?;
	let graph_rag = support::find_by_field(
		scenarios,
		"/scenario_id",
		"graph_rag_citation_navigation_knowledge_surfaces",
	)?;
	let private_provider =
		support::find_by_field(scenarios, "/scenario_id", "private_provider_production_gates")?;

	assert_eq!(qmd.pointer("/current_outcome").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(qmd.pointer("/current_status").and_then(Value::as_str), Some("pass"));
	assert!(qmd.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("17 pass")
			&& evidence.contains("13 wrong_result")
			&& evidence.contains("does not retest or erase")
	}));
	assert_eq!(mem0.pointer("/current_outcome").and_then(Value::as_str), Some("unchanged"));
	assert!(mem0.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("mem0/OpenMemory local OSS history")
			&& evidence.contains("OpenMemory UI/export remains setup-blocked")
	}));
	assert_eq!(letta.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		graph_rag.pointer("/current_status").and_then(Value::as_str),
		Some("typed_non_pass")
	);
	assert!(graph_rag.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("0 pass")
			&& evidence.contains("1 wrong_result")
			&& evidence.contains("3 blocked")
	}));
	assert_eq!(private_provider.pointer("/follow_up").and_then(Value::as_str), Some("XY-930"));

	Ok(())
}

fn assert_xy955_optimization_queue(report: &Value) -> color_eyre::Result<()> {
	let queue = support::array_at(report, "/optimization_queue")?;
	let qmd = support::find_by_field(queue, "/issue", "XY-923")?;
	let private_provider = support::find_by_field(queue, "/issue", "XY-930")?;
	let openviking = support::find_by_field(queue, "/issue", "XY-928")?;
	let letta = support::find_by_field(queue, "/issue", "letta-core-archive-adapter-brief")?;
	let service_native =
		support::find_by_field(queue, "/issue", "service-native-dreaming-outputs-brief")?;

	assert_eq!(qmd.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(private_provider.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(openviking.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(letta.pointer("/status").and_then(Value::as_str), Some("proposed"));
	assert_eq!(service_native.pointer("/status").and_then(Value::as_str), Some("proposed"));
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not treat qmd full-suite wrong_result counts as a regression of qmd debug ergonomics."
	)?);

	Ok(())
}

fn assert_xy955_follow_up_issue_briefs(report: &Value) -> color_eyre::Result<()> {
	let existing = support::array_at(report, "/follow_up_issue_briefs/existing")?;
	let proposed = support::array_at(report, "/follow_up_issue_briefs/proposed")?;
	let qmd = support::find_by_field(existing, "/issue", "XY-923")?;
	let private_provider = support::find_by_field(existing, "/issue", "XY-930")?;
	let letta = support::find_by_field(proposed, "/issue", "letta-core-archive-adapter-brief")?;
	let service_native =
		support::find_by_field(proposed, "/issue", "service-native-dreaming-outputs-brief")?;

	assert!(qmd.pointer("/scope").and_then(Value::as_str).is_some_and(|scope| {
		scope.contains("immediate top-k") && scope.contains("candidate-drop artifacts")
	}));
	assert!(qmd.pointer("/non_goal").and_then(Value::as_str).is_some_and(|non_goal| {
		non_goal.contains("qmd full-suite wrong_result counts")
			&& non_goal.contains("debug ergonomics")
	}));
	assert!(
		private_provider
			.pointer("/non_goal")
			.and_then(Value::as_str)
			.is_some_and(|non_goal| non_goal.contains("Do not infer credentials"))
	);
	assert!(letta.pointer("/validation").and_then(Value::as_str).is_some_and(|validation| {
		validation.contains("Letta core block JSON") && validation.contains("typed outcome states")
	}));
	assert!(
		service_native
			.pointer("/non_goal")
			.and_then(Value::as_str)
			.is_some_and(|non_goal| non_goal.contains("Pulse clone"))
	);

	Ok(())
}
