use std::fs;

use color_eyre::{Result, eyre};
use serde_json::Value;

use super::support::*;

#[test]
fn qmd_openviking_strength_profile_report_preserves_claim_boundaries() -> Result<()> {
	let report =
		serde_json::from_str::<Value>(&fs::read_to_string(strength_profile_report_path()?)?)?;
	let markdown = fs::read_to_string(strength_profile_markdown_path()?)?;
	let readme = fs::read_to_string(readme_path()?)?;
	let benchmarking_index = fs::read_to_string(benchmarking_index_path()?)?;
	let iteration_direction = fs::read_to_string(iteration_direction_report_path()?)?;

	assert_strength_profile_summary(&report);
	assert_strength_profile_terms(&report)?;
	assert_qmd_strength_profile(&report)?;
	assert_qmd_wrong_result_diagnosis(&report)?;
	assert_openviking_strength_profile(&report)?;
	assert_strength_profile_json_claim_boundaries(&report)?;
	assert_strength_profile_markdown_boundaries(&markdown);
	assert_operator_facing_strength_profile_boundaries(
		&readme,
		&benchmarking_index,
		&iteration_direction,
	);

	Ok(())
}

#[test]
fn current_benchmark_reports_preserve_live_sweep_boundaries() -> Result<()> {
	let measurement_audit = fs::read_to_string(measurement_coverage_audit_path()?)?;
	let measurement_audit_json = serde_json::from_str::<Value>(&fs::read_to_string(
		measurement_coverage_audit_json_path()?,
	)?)?;
	let competitor_matrix = fs::read_to_string(competitor_strength_matrix_path()?)?;
	let competitor_matrix_json = serde_json::from_str::<Value>(&fs::read_to_string(
		competitor_strength_matrix_json_path()?,
	)?)?;
	let iteration_direction = fs::read_to_string(iteration_direction_report_path()?)?;
	let external_manifest = fs::read_to_string(external_adapter_manifest_path())?;
	let comparison_external_projects = fs::read_to_string(comparison_external_projects_path()?)?;
	let retrieval_debug_profile =
		serde_json::from_str::<Value>(&fs::read_to_string(retrieval_debug_profile_json_path()?)?)?;
	let temporal_history = serde_json::from_str::<Value>(&fs::read_to_string(
		temporal_history_competitor_gap_json_path()?,
	)?)?;

	assert_current_report_text_boundaries(
		&measurement_audit,
		&competitor_matrix,
		&iteration_direction,
		&external_manifest,
		&comparison_external_projects,
	);

	assert!(competitor_matrix.contains("claude-mem work_resume remains `not_encoded`"));
	assert!(!competitor_matrix.contains("claude-mem `wrong_result`, OpenViking work_resume"));

	let qmd_live = find_by_field(
		array_at(&measurement_audit_json, "/live_real_world_adapters")?,
		"/adapter",
		"qmd live CLI adapter",
	)?;

	assert_eq!(qmd_live.pointer("/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(qmd_live.pointer("/wrong_result").and_then(Value::as_u64), Some(6));
	assert_eq!(qmd_live.pointer("/expected_evidence_matched").and_then(Value::as_u64), Some(38));
	assert_eq!(qmd_live.pointer("/evidence_covered_count").and_then(Value::as_u64), Some(45));

	let memory_evolution = find_by_field(
		array_at(&measurement_audit_json, "/live_suite_breakdown")?,
		"/suite",
		"memory_evolution",
	)?;

	assert_eq!(
		memory_evolution.pointer("/elf_status_counts/wrong_result").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		memory_evolution.pointer("/qmd_status_counts/wrong_result").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		retrieval_debug_profile
			.pointer("/live_real_world_full_sweep_context/qmd/pass")
			.and_then(Value::as_u64),
		Some(17)
	);
	assert_eq!(
		retrieval_debug_profile
			.pointer("/live_real_world_full_sweep_context/qmd/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);

	assert_competitor_strength_matrix_json(&competitor_matrix_json)?;

	let openmemory_command = find_by_field(
		array_at(&temporal_history, "/commands")?,
		"/command",
		"cargo make openmemory-ui-export-readback",
	)?;

	assert!(
		openmemory_command
			.pointer("/artifact")
			.and_then(Value::as_str)
			.is_some_and(|artifact| artifact.contains("tmp/live-baseline/mem0-checks.json")
				&& artifact.contains("tmp/live-baseline/mem0-openmemory-ui-export.json"))
	);

	Ok(())
}

fn assert_current_report_text_boundaries(
	measurement_audit: &str,
	competitor_matrix: &str,
	iteration_direction: &str,
	external_manifest: &str,
	comparison_external_projects: &str,
) {
	assert!(
		measurement_audit.contains(
			"| `memory_evolution` | `6` | `pass:1`, `wrong_result:5` | `wrong_result:6` |"
		)
	);
	assert!(
		measurement_audit
			.contains("qmd live fails 6/6 jobs after missing the delete/TTL tombstone evidence")
	);
	assert!(measurement_audit.contains("Basic local smoke and local OSS history/readback pass"));
	assert!(measurement_audit.contains("claude-mem hook/viewer capture is `blocked`"));
	assert!(!measurement_audit.contains("claude-mem hook/viewer capture remains untested"));
	assert!(!measurement_audit.contains("blocked or untested"));

	assert_measurement_audit_adapter_status_counts(measurement_audit);

	assert!(
		competitor_matrix
			.contains("broader live suites remain `wrong_result`, `blocked`, or `not_encoded`")
	);
	assert!(competitor_matrix.contains(
		"Overall adapter-status counts: 4 `pass`,\n6 `wrong_result`, 1 `lifecycle_fail`, 7 `blocked`, and 5 `not_encoded`."
	));
	assert!(!competitor_matrix.contains("5 `blocked`, and 7 `not_encoded`"));
	assert!(
		competitor_matrix
			.contains("mem0/OpenMemory local OSS entity-scoped personalization now passes")
	);
	assert!(competitor_matrix.contains("scoped preference behavior is a measured tie"));
	assert!(
		!competitor_matrix.contains("mem0/OpenMemory and Letta personalization are `not_encoded`")
	);
	assert!(external_manifest.contains(
		"The record is a full-suite sweep, not a full-suite pass; wrong_result, blocked, and not_encoded states remain visible."
	));
	assert!(external_manifest.contains(
		"The qmd live real-world sweep covers the current encoded fixture corpus; expanded retrieval-debug strength suites still need their own materialized adapter run."
	));
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for scoped local OSS same-corpus retrieval")
	);
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for local same-corpus retrieval, reindex/update/delete")
	);
	assert!(iteration_direction.contains("| Jobs | `55` |"));
	assert!(iteration_direction.contains("| Encoded suites | `15` |"));
	assert!(iteration_direction.contains("| Pass | `49` |"));
	assert!(iteration_direction.contains("| Evidence coverage | `123/123` |"));
	assert!(iteration_direction.contains("| Expected evidence recall | `115/115` |"));

	for stale_phrase in [
		"same live sweep shape as ELF",
		"ELF and qmd live fail 5/6 jobs",
		"both systems currently fail 5/6 live memory-evolution jobs",
		"wrong_result, incomplete, blocked, and not_encoded states remain visible",
		"broader live suites remain `wrong_result`, `incomplete`, or `not_encoded`",
		"The qmd live real-world slice covers representative jobs only",
		"| Jobs | `40` |",
		"| Encoded suites | `11` |",
		"| Jobs | `50` |",
		"| Encoded suites | `14` |",
		"| Pass | `38` |",
		"| Pass | `45` |",
		"| Evidence coverage | `115/115` |",
		"| Expected evidence recall | `107/107` |",
		"history/UI/hosted/graph behavior remains",
		"current local adapter is incomplete/wrong-result",
		"current adapter is incomplete/invalid-result",
	] {
		assert!(!measurement_audit.contains(stale_phrase));
		assert!(!competitor_matrix.contains(stale_phrase));
		assert!(!iteration_direction.contains(stale_phrase));
		assert!(!external_manifest.contains(stale_phrase));
		assert!(!comparison_external_projects.contains(stale_phrase));
	}
}

fn assert_competitor_strength_matrix_json(matrix: &Value) -> Result<()> {
	let projects = array_at(matrix, "/project_matrix")?;
	let scenarios = array_at(matrix, "/scenario_matrix")?;

	assert_competitor_strength_matrix_manifest_counts(matrix);
	assert_competitor_strength_matrix_project_json(projects)?;
	assert_competitor_strength_matrix_scenario_json(scenarios)?;

	Ok(())
}

fn assert_competitor_strength_matrix_project_json(projects: &[Value]) -> Result<()> {
	let qmd = find_by_field(projects, "/project", "qmd")?;
	let mem0 = find_by_field(projects, "/project", "mem0/OpenMemory")?;
	let claude_mem = find_by_field(projects, "/project", "claude-mem")?;
	let openviking = find_by_field(projects, "/project", "OpenViking")?;

	assert_eq!(
		qmd.pointer("/current_evidence_class").and_then(Value::as_str),
		Some("live_real_world")
	);
	assert_eq!(qmd.pointer("/measured_status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		qmd.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert!(qmd.pointer("/benchmark_before_claim").and_then(Value::as_str).is_some_and(|claim| {
		claim.contains("Keep qmd deep retrieval/debug profiling separate")
			&& claim.contains("narrow operator-debug live slice")
	}));
	assert!(
		qmd.pointer("/borrow_if_stronger")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("transparent local knobs"))
	);
	assert_eq!(mem0.pointer("/measured_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		mem0.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		mem0.pointer("/unsupported_or_blocked_status/typed_reason").and_then(Value::as_str),
		Some("openmemory_export_helper_setup_blocked")
	);
	assert!(
		mem0.pointer("/benchmark_before_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("OpenMemory product app import/export"))
	);
	assert!(
		claude_mem
			.pointer("/unsupported_or_blocked_status/details")
			.and_then(Value::as_str)
			.is_some_and(|details| details.contains("rerun/inspection targets")
				&& details.contains("tmp/live-baseline/claude-mem-checks.json"))
	);
	assert_eq!(
		openviking.pointer("/current_evidence_class").and_then(Value::as_str),
		Some("live_baseline_only")
	);
	assert_eq!(
		openviking.pointer("/measured_status").and_then(Value::as_str),
		Some("wrong_result")
	);
	assert_eq!(
		openviking.pointer("/unsupported_or_blocked_status/state").and_then(Value::as_str),
		Some("blocked")
	);
	assert!(
		openviking
			.pointer("/unsupported_or_blocked_status/details")
			.and_then(Value::as_str)
			.is_some_and(|details| details.contains("encoded as blocked fixtures"))
	);
	assert!(
		openviking
			.pointer("/benchmark_before_claim")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("evidence-bearing same-corpus output pass"))
	);

	Ok(())
}

fn assert_competitor_strength_matrix_scenario_json(scenarios: &[Value]) -> Result<()> {
	let retrieval_debug = find_by_field(scenarios, "/scenario_id", "retrieval_debug")?;
	let work_resume = find_by_field(scenarios, "/scenario_id", "work_resume")?;
	let operator_debug = find_by_field(scenarios, "/scenario_id", "operator_debugging")?;
	let context_trajectory = find_by_field(scenarios, "/scenario_id", "context_trajectory")?;
	let consolidation = find_by_field(scenarios, "/scenario_id", "consolidation")?;

	assert!(
		retrieval_debug
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("Measured tie on encoded retrieval answers"))
	);
	assert!(retrieval_debug.pointer("/current_state").and_then(Value::as_str).is_some_and(
		|state| state.contains("qmd remains stronger on local debug ergonomics not fully scored")
	));
	assert!(
		work_resume
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("claude-mem work_resume remains not_encoded")
				&& !claim.contains("claude-mem is wrong_result"))
	);
	assert!(
		operator_debug
			.pointer("/current_elf_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("narrow live_real_world operator-debug slice"))
	);
	assert!(
		operator_debug
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd now has a narrow live_real_world"))
	);
	assert!(
		operator_debug
			.pointer("/next_measurement")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("OpenMemory and claude-mem UI/export"))
	);
	assert!(
		consolidation
			.pointer("/current_elf_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("XY-934 adds live_real_world")
				&& claim.contains("zero source mutations"))
	);
	assert!(
		consolidation
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim.contains("qmd remains not_encoded")
				&& claim.contains("product references only"))
	);

	let personalization = find_by_field(scenarios, "/scenario_id", "personalization")?;

	assert_personalization_matrix_record(personalization);

	assert!(
		context_trajectory
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("not a measured live winner"))
	);
	assert!(
		context_trajectory
			.pointer("/next_measurement")
			.and_then(Value::as_str)
			.is_some_and(|measurement| measurement.contains("evidence-bearing retrieval pass"))
	);

	Ok(())
}

fn assert_personalization_matrix_record(personalization: &Value) {
	assert!(
		personalization
			.pointer("/current_competitor_evidence")
			.and_then(Value::as_str)
			.is_some_and(|claim| claim
				.contains("mem0/OpenMemory local OSS entity-scoped personalization now passes")
				&& claim.contains("Letta personalization is research_gate not_encoded"))
	);
	assert!(
		personalization
			.pointer("/current_state")
			.and_then(Value::as_str)
			.is_some_and(|state| state.contains("scoped personalization is a tie"))
	);
}

fn assert_competitor_strength_matrix_manifest_counts(matrix: &Value) {
	assert_eq!(
		matrix.pointer("/manifest_summary/adapter_records").and_then(Value::as_u64),
		Some(23)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/evidence_class_counts/live_real_world")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		matrix.pointer("/manifest_summary/overall_status_counts/pass").and_then(Value::as_u64),
		Some(4)
	);
	assert_eq!(
		matrix.pointer("/manifest_summary/overall_status_counts/blocked").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/overall_status_counts/not_encoded")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		matrix
			.pointer("/manifest_summary/overall_status_counts/wrong_result")
			.and_then(Value::as_u64),
		Some(6)
	);
}

fn assert_strength_profile_summary(report: &Value) {
	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.competitor_strength_profile_report/v1")
	);
	assert_eq!(
		report.pointer("/summary/qmd/retrieval_quality").and_then(Value::as_str),
		Some("tie")
	);
	assert_eq!(
		report.pointer("/summary/qmd/local_query_transparency").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/qmd/local_replayability").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/qmd/overall_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report.pointer("/summary/openviking/overall_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/elf_win")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/qmd_strength_profile/win_tie_loss_summary/tie").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/elf_loss")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/qmd_strength_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/not_tested")
			.and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report
			.pointer("/openviking_context_trajectory_profile/win_tie_loss_summary/elf_win")
			.and_then(Value::as_u64),
		Some(1)
	);
}

fn assert_strength_profile_terms(report: &Value) -> Result<()> {
	let result_terms = array_at(report, "/result_type_terms")?;
	let coverage_terms = array_at(report, "/coverage_status_terms")?;
	let outcome_terms = array_at(report, "/outcome_terms")?;
	let actual_result_terms = string_array_at(report, "/result_type_terms")?;
	let actual_coverage_terms = string_array_at(report, "/coverage_status_terms")?;

	assert_eq!(
		actual_result_terms,
		[
			"pass",
			"wrong_result",
			"blocked",
			"incomplete",
			"lifecycle_fail",
			"not_encoded",
			"unsupported_claim",
		]
		.map(str::to_owned)
	);
	assert_eq!(
		actual_coverage_terms,
		[
			"pass",
			"wrong_result",
			"blocked",
			"incomplete",
			"lifecycle_fail",
			"not_encoded",
			"unsupported",
			"unsupported_claim",
		]
		.map(str::to_owned)
	);
	assert!(!result_terms.iter().any(|term| term.as_str() == Some("unsupported")));
	assert!(!result_terms.iter().any(|term| term.as_str() == Some("partial")));
	assert!(!coverage_terms.iter().any(|term| term.as_str() == Some("partial")));
	assert!(result_terms.iter().any(|term| term.as_str() == Some("unsupported_claim")));
	assert!(coverage_terms.iter().any(|term| term.as_str() == Some("unsupported")));

	assert_value_in_terms(report, "/summary/qmd/overall_outcome", outcome_terms)?;
	assert_value_in_terms(report, "/summary/openviking/overall_outcome", outcome_terms)?;

	for scenario in array_at(report, "/qmd_strength_profile/scenario_outcomes")? {
		assert_value_in_terms(scenario, "/result_type", result_terms)?;
		assert_value_in_terms(scenario, "/elf_status", coverage_terms)?;
		assert_value_in_terms(scenario, "/qmd_status", coverage_terms)?;
	}
	for scenario in array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")? {
		assert_value_in_terms(scenario, "/result_type", result_terms)?;
		assert_value_in_terms(scenario, "/openviking_status", coverage_terms)?;
		assert_value_in_terms(scenario, "/elf_equivalent_status", coverage_terms)?;
	}

	Ok(())
}

fn assert_value_in_terms(value: &Value, pointer: &str, terms: &[Value]) -> Result<()> {
	let actual = value
		.pointer(pointer)
		.and_then(Value::as_str)
		.ok_or_else(|| eyre::eyre!("missing string at {pointer}"))?;

	assert!(
		terms.iter().any(|term| term.as_str() == Some(actual)),
		"{actual} at {pointer} is not declared in the report term list"
	);

	Ok(())
}

fn assert_qmd_strength_profile(report: &Value) -> Result<()> {
	let qmd_scenarios = array_at(report, "/qmd_strength_profile/scenario_outcomes")?;
	let local_transparency =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-local-query-transparency")?;
	let retrieval = find_by_field(qmd_scenarios, "/scenario_id", "qmd-retrieval-quality")?;
	let rerank_controls =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-expansion-fusion-rerank-controls")?;
	let stale_isolation =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-stale-context-isolation")?;
	let lifecycle = find_by_field(qmd_scenarios, "/scenario_id", "qmd-update-delete-cold-start")?;
	let operator_debug =
		find_by_field(qmd_scenarios, "/scenario_id", "qmd-operator-debug-evidence")?;
	let replayability = find_by_field(qmd_scenarios, "/scenario_id", "qmd-local-replayability")?;
	let wrong_result = find_by_field(qmd_scenarios, "/scenario_id", "qmd-wrong-result-diagnosis")?;

	assert_eq!(qmd_scenarios.len(), 8);
	assert_eq!(retrieval.pointer("/elf_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(
		local_transparency.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(
		local_transparency.pointer("/result_type").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(
		rerank_controls.pointer("/result_type").and_then(Value::as_str),
		Some("not_encoded")
	);
	assert_eq!(stale_isolation.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(stale_isolation.pointer("/elf_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(lifecycle.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(lifecycle.pointer("/elf_outcome").and_then(Value::as_str), Some("tie"));
	assert_eq!(operator_debug.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(operator_debug.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(replayability.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));
	assert_eq!(replayability.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(
		wrong_result.pointer("/evidence_class").and_then(Value::as_str),
		Some("research_gate")
	);
	assert_eq!(wrong_result.pointer("/result_type").and_then(Value::as_str), Some("not_encoded"));

	Ok(())
}

fn assert_qmd_wrong_result_diagnosis(report: &Value) -> Result<()> {
	let taxonomy = array_at(report, "/qmd_strength_profile/wrong_result_diagnosis/taxonomy")?;
	let absent = find_by_field(taxonomy, "/class", "evidence_absent")?;
	let dropped = find_by_field(taxonomy, "/class", "retrieved_but_dropped")?;
	let narrated = find_by_field(taxonomy, "/class", "selected_but_not_narrated")?;
	let lifecycle = find_by_field(taxonomy, "/class", "contradicted_by_lifecycle_evidence")?;

	assert_eq!(absent.pointer("/coverage").and_then(Value::as_str), Some("observed"));
	assert_eq!(
		dropped.pointer("/coverage").and_then(Value::as_str),
		Some("not_observed_candidate_trace_missing")
	);
	assert_eq!(narrated.pointer("/coverage").and_then(Value::as_str), Some("observed"));
	assert_eq!(lifecycle.pointer("/coverage").and_then(Value::as_str), Some("observed"));

	let qmd_diagnosis_jobs = array_at(report, "/qmd_strength_profile/wrong_result_diagnosis/jobs")?;
	let delete_job =
		find_by_field(qmd_diagnosis_jobs, "/job_id", "memory-evolution-delete-ttl-001")?;

	assert_eq!(qmd_diagnosis_jobs.len(), 6);
	assert_eq!(delete_job.pointer("/qmd_status").and_then(Value::as_str), Some("wrong_result"));
	assert!(array_contains_str(delete_job, "/missing_evidence", "delete-tombstone")?);
	assert!(
		delete_job
			.pointer("/diagnosis")
			.and_then(Value::as_str)
			.is_some_and(|diagnosis| diagnosis.contains("typed wrong_result"))
	);

	Ok(())
}

fn assert_openviking_strength_profile(report: &Value) -> Result<()> {
	let openviking_scenarios =
		array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")?;
	let trajectory = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-staged-retrieval-trajectory",
	)?;
	let precondition = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-evidence-bearing-retrieval-precondition",
	)?;
	let local_embed_setup =
		find_by_field(openviking_scenarios, "/scenario_id", "openviking-local-embed-setup")?;
	let missed_terms = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-missed-expected-terms-evidence",
	)?;
	let hierarchy =
		find_by_field(openviking_scenarios, "/scenario_id", "openviking-hierarchy-selection")?;
	let recursive_expansion = find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-recursive-context-expansion",
	)?;

	assert_eq!(openviking_scenarios.len(), 6);
	assert_eq!(
		trajectory.pointer("/evidence_class").and_then(Value::as_str),
		Some("fixture_backed")
	);
	assert_eq!(trajectory.pointer("/result_type").and_then(Value::as_str), Some("blocked"));
	assert_eq!(trajectory.pointer("/openviking_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(local_embed_setup.pointer("/result_type").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		local_embed_setup.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);
	assert_eq!(local_embed_setup.pointer("/typed_blocker"), Some(&Value::Null));
	assert_eq!(precondition.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(precondition.pointer("/elf_outcome").and_then(Value::as_str), Some("elf_win"));
	assert_eq!(
		precondition.pointer("/typed_blocker").and_then(Value::as_str),
		Some("output_missed_expected_terms")
	);
	assert_eq!(missed_terms.pointer("/result_type").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(missed_terms.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(hierarchy.pointer("/result_type").and_then(Value::as_str), Some("blocked"));
	assert_eq!(hierarchy.pointer("/elf_outcome").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(
		recursive_expansion.pointer("/result_type").and_then(Value::as_str),
		Some("blocked")
	);
	assert_eq!(
		recursive_expansion.pointer("/elf_outcome").and_then(Value::as_str),
		Some("not_tested")
	);

	Ok(())
}

fn assert_strength_profile_json_claim_boundaries(report: &Value) -> Result<()> {
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not broadly beat qmd; it ties encoded retrieval and lifecycle correctness, keeps qmd query transparency as not_tested for comparative scoring, and leaves replayability not_tested."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"qmd expansion, fusion, and rerank superiority remains not_tested because the current qmd paths use --no-rerank and do not score internals."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not beat OpenViking on context trajectory; OpenViking trajectory strengths remain blocked/not_tested behind a wrong_result same-corpus output precondition and missing staged artifacts."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"Research_gate and blocked fixture records are follow-up gates, not pass evidence."
	)?);
	assert!(array_contains_str(
		report,
		"/claim_boundaries",
		"Missing equivalent surfaces are encoded as unsupported, blocked, or not_encoded rather than fake losses."
	)?);

	Ok(())
}

fn assert_strength_profile_markdown_boundaries(markdown: &str) {
	assert!(
		markdown.contains(
			"| Wrong-result diagnosis | `research_gate` | `not_encoded` | `not_tested` |"
		)
	);
	assert!(
		markdown.contains("ELF ties qmd on the current encoded retrieval-correctness surfaces")
	);
	assert!(markdown.contains("qmd remains the local retrieval-debug UX reference"));
	assert!(markdown.contains("not scored as comparative ELF wins or losses"));
	assert!(markdown.contains("ELF currently wins only the equivalent OpenViking same-corpus"));
	assert!(markdown.contains("Do not claim ELF broadly beats qmd"));
	assert!(markdown.contains(
		"Do not claim ELF beats OpenViking on staged retrieval, hierarchy, or recursive"
	));
	assert!(markdown.contains(
		"Do not turn `research_gate`, `blocked`, `not_encoded`, or `unsupported` surfaces"
	));
	assert!(markdown.contains("no pass evidence is claimed"));
	assert!(markdown.contains("typed `wrong_result` state"));
}

fn assert_operator_facing_strength_profile_boundaries(
	readme: &str,
	benchmarking_index: &str,
	iteration_direction: &str,
) {
	assert!(readme.contains("Full-suite live real-world adapter sweep after XY-926"));
	assert!(readme.contains("all 55 checked-in jobs across 13 suites"));
	assert!(readme.contains("ELF now live-scores capture/write-policy"));
	assert!(readme.contains("consolidation proposal review"));
	assert!(readme.contains("knowledge-page rebuild/lint"));
	assert!(readme.contains("operator-debugging fixtures"));
	assert!(!readme.contains("memory-evolution wrong results"));
	assert!(readme.contains("Live temporal reconciliation after XY-905"));
	assert!(readme.contains("now reports ELF live `memory_evolution` as 6/6 pass"));
	assert!(readme.contains("broad qmd, Graphiti/Zep, mem0/OpenMemory, Letta"));
	assert!(readme.contains("production-ops operator boundaries"));
	assert!(readme.contains("core/archival live adapter gap"));
	assert!(collapse_whitespace(readme).contains("blocked context-trajectory measurement"));
	assert!(
		readme
			.contains("consolidation, knowledge, capture, and core/archival typed non-pass states")
	);
	assert!(readme.contains("operator-debug trace hydration"));
	assert!(readme.contains("qmd remains the local retrieval-debug UX reference"));
	assert!(readme.contains("broad ELF-over-qmd"));
	assert!(readme.contains("qmd and OpenViking Strength-Profile Report - June 11, 2026"));
	assert!(benchmarking_index.contains("2026-06-11-qmd-openviking-strength-profile-report.md"));
	assert!(
		benchmarking_index.contains("separates qmd retrieval quality from debug/replay ergonomics")
	);
	assert!(benchmarking_index.contains("preserves XY-928 OpenViking"));
	assert!(
		benchmarking_index
			.contains("context-trajectory surfaces as blocked/not-tested until scored staged")
	);
	assert!(
		iteration_direction
			.contains("ELF and qmd are tied on the encoded live retrieval, work-resume, and")
	);
	assert!(iteration_direction.contains("ELF does not yet beat qmd's local retrieval-debug"));

	assert_iteration_direction_current_measurement_counts(iteration_direction);

	assert!(iteration_direction.contains(
		"ELF beats OpenViking on context trajectory. The scenario is encoded as blocked"
	));
	assert!(
		iteration_direction
			.contains("Do not promote a reference project into a win/loss claim until")
	);
}

fn assert_measurement_audit_adapter_status_counts(markdown: &str) {
	for expected in [
		"| `blocked` | `7` |",
		"| `not_encoded` | `5` |",
		"The generated JSON report emits `external_project_count: 16`",
	] {
		assert!(markdown.contains(expected), "missing measurement audit text: {expected}");
	}
	for stale in ["| `blocked` | `6` |", "| `not_encoded` | `6` |"] {
		assert!(!markdown.contains(stale), "stale measurement audit text: {stale}");
	}
}

fn assert_iteration_direction_current_measurement_counts(markdown: &str) {
	for expected in [
		"| Jobs | `55` |",
		"| Encoded suites | `15` |",
		"| Blocked | `6` |",
		"| Mean score | `0.891` |",
		"| Evidence coverage | `123/123` |",
		"| Source-ref coverage | `123/123` |",
		"| Quote coverage | `123/123` |",
		"| Expected evidence recall | `115/115` |",
		"| `blocked` | `7` |",
		"| `not_encoded` | `5` |",
		"`live_baseline_only`, `fixture_backed`, and `research_gate`",
		"`blocked` for fixture-backed trajectory gates",
	] {
		assert!(markdown.contains(expected), "missing iteration-direction text: {expected}");
	}
	for stale in [
		"| Jobs | `40` |",
		"| Encoded suites | `11` |",
		"| Jobs | `50` |",
		"| Encoded suites | `14` |",
		"| Mean score | `0.950` |",
		"| Mean score | `0.900` |",
		"| Evidence coverage | `88/88` |",
		"| Evidence coverage | `115/115` |",
		"| Expected evidence recall | `80/80` |",
		"| Expected evidence recall | `107/107` |",
		"| `blocked` | `5` |",
		"| `not_encoded` | `7` |",
		"`live_baseline_only` plus `research_gate`",
	] {
		assert!(!markdown.contains(stale), "stale iteration-direction text: {stale}");
	}
}
