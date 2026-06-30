use std::fs;

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn qmd_openviking_strength_profile_report_preserves_claim_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::strength_profile_report_path()?,
	)?)?;
	let markdown = fs::read_to_string(support::strength_profile_markdown_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let iteration_direction = fs::read_to_string(support::iteration_direction_report_path()?)?;

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
	let result_terms = support::array_at(report, "/result_type_terms")?;
	let coverage_terms = support::array_at(report, "/coverage_status_terms")?;
	let outcome_terms = support::array_at(report, "/outcome_terms")?;
	let actual_result_terms = support::string_array_at(report, "/result_type_terms")?;
	let actual_coverage_terms = support::string_array_at(report, "/coverage_status_terms")?;

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

	for scenario in support::array_at(report, "/qmd_strength_profile/scenario_outcomes")? {
		assert_value_in_terms(scenario, "/result_type", result_terms)?;
		assert_value_in_terms(scenario, "/elf_status", coverage_terms)?;
		assert_value_in_terms(scenario, "/qmd_status", coverage_terms)?;
	}
	for scenario in
		support::array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")?
	{
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
	let qmd_scenarios = support::array_at(report, "/qmd_strength_profile/scenario_outcomes")?;
	let local_transparency =
		support::find_by_field(qmd_scenarios, "/scenario_id", "qmd-local-query-transparency")?;
	let retrieval = support::find_by_field(qmd_scenarios, "/scenario_id", "qmd-retrieval-quality")?;
	let rerank_controls = support::find_by_field(
		qmd_scenarios,
		"/scenario_id",
		"qmd-expansion-fusion-rerank-controls",
	)?;
	let stale_isolation =
		support::find_by_field(qmd_scenarios, "/scenario_id", "qmd-stale-context-isolation")?;
	let lifecycle =
		support::find_by_field(qmd_scenarios, "/scenario_id", "qmd-update-delete-cold-start")?;
	let operator_debug =
		support::find_by_field(qmd_scenarios, "/scenario_id", "qmd-operator-debug-evidence")?;
	let replayability =
		support::find_by_field(qmd_scenarios, "/scenario_id", "qmd-local-replayability")?;
	let wrong_result =
		support::find_by_field(qmd_scenarios, "/scenario_id", "qmd-wrong-result-diagnosis")?;

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
	let taxonomy =
		support::array_at(report, "/qmd_strength_profile/wrong_result_diagnosis/taxonomy")?;
	let absent = support::find_by_field(taxonomy, "/class", "evidence_absent")?;
	let dropped = support::find_by_field(taxonomy, "/class", "retrieved_but_dropped")?;
	let narrated = support::find_by_field(taxonomy, "/class", "selected_but_not_narrated")?;
	let lifecycle =
		support::find_by_field(taxonomy, "/class", "contradicted_by_lifecycle_evidence")?;

	assert_eq!(absent.pointer("/coverage").and_then(Value::as_str), Some("observed"));
	assert_eq!(
		dropped.pointer("/coverage").and_then(Value::as_str),
		Some("not_observed_candidate_trace_missing")
	);
	assert_eq!(narrated.pointer("/coverage").and_then(Value::as_str), Some("observed"));
	assert_eq!(lifecycle.pointer("/coverage").and_then(Value::as_str), Some("observed"));

	let qmd_diagnosis_jobs =
		support::array_at(report, "/qmd_strength_profile/wrong_result_diagnosis/jobs")?;
	let delete_job =
		support::find_by_field(qmd_diagnosis_jobs, "/job_id", "memory-evolution-delete-ttl-001")?;

	assert_eq!(qmd_diagnosis_jobs.len(), 6);
	assert_eq!(delete_job.pointer("/qmd_status").and_then(Value::as_str), Some("wrong_result"));
	assert!(support::array_contains_str(delete_job, "/missing_evidence", "delete-tombstone")?);
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
		support::array_at(report, "/openviking_context_trajectory_profile/scenario_outcomes")?;
	let trajectory = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-staged-retrieval-trajectory",
	)?;
	let precondition = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-evidence-bearing-retrieval-precondition",
	)?;
	let local_embed_setup = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-local-embed-setup",
	)?;
	let missed_terms = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-missed-expected-terms-evidence",
	)?;
	let hierarchy = support::find_by_field(
		openviking_scenarios,
		"/scenario_id",
		"openviking-hierarchy-selection",
	)?;
	let recursive_expansion = support::find_by_field(
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
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not broadly beat qmd; it ties encoded retrieval and lifecycle correctness, keeps qmd query transparency as not_tested for comparative scoring, and leaves replayability not_tested."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"qmd expansion, fusion, and rerank superiority remains not_tested because the current qmd paths use --no-rerank and do not score internals."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not beat OpenViking on context trajectory; OpenViking trajectory strengths remain blocked/not_tested behind a wrong_result same-corpus output precondition and missing staged artifacts."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"Research_gate and blocked fixture records are follow-up gates, not pass evidence."
	)?);
	assert!(support::array_contains_str(
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
	assert!(
		support::collapse_whitespace(readme).contains("blocked context-trajectory measurement")
	);
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
