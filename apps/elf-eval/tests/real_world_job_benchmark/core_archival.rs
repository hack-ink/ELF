use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn core_archival_memory_fixtures_score_separate_core_and_archival_jobs() -> Result<()> {
	let report = support::run_json_report_from(support::core_archival_memory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/evidence_required_count").and_then(Value::as_u64),
		Some(14)
	);
	assert_eq!(report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64), Some(14));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/scope_violation_count").and_then(Value::as_u64), Some(0));

	let suites = support::array_at(&report, "/suites")?;
	let core = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(core.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let jobs = support::array_at(&report, "/jobs")?;

	for job_id in [
		"core-archival-core-block-attachment-001",
		"core-archival-core-block-scope-001",
		"core-archival-core-block-provenance-001",
		"core-archival-stale-core-detection-001",
		"core-archival-archival-fallback-001",
		"core-archival-project-decision-recovery-001",
	] {
		let job = support::find_by_field(jobs, "/job_id", job_id)?;

		assert_eq!(job.pointer("/suite_id").and_then(Value::as_str), Some("core_archival_memory"));
		assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	}

	let scope = support::find_by_field(jobs, "/job_id", "core-archival-core-block-scope-001")?;
	let decision =
		support::find_by_field(jobs, "/job_id", "core-archival-project-decision-recovery-001")?;

	assert_eq!(scope.pointer("/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(scope.pointer("/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(scope.pointer("/scope_violation_count").and_then(Value::as_u64), Some(0));
	assert!(
		decision
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|content| content.contains("Letta remains blocked or not_tested"))
	);
	assert!(
		support::array_at(decision, "/produced_evidence")?
			.iter()
			.any(|id| id.as_str() == Some("decision-letta-export-boundary"))
	);

	Ok(())
}

#[test]
fn memory_authority_benchmark_covers_entity_history_and_core_archive_strengths() -> Result<()> {
	let report = support::run_json_report_from(support::real_world_memory_fixture_dir())?;

	assert_eq!(
		report.pointer("/summary/history_readback_encoded_count").and_then(Value::as_u64),
		Some(4)
	);

	let suites = support::array_at(&report, "/suites")?;
	let memory_evolution = support::find_by_field(suites, "/suite_id", "memory_evolution")?;
	let core_archival = support::find_by_field(suites, "/suite_id", "core_archival_memory")?;

	assert_eq!(memory_evolution.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(core_archival.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		memory_evolution.pointer("/history_readback_encoded_count").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(core_archival.pointer("/encoded_job_count").and_then(Value::as_u64), Some(6));

	let jobs = support::array_at(&report, "/jobs")?;
	let preference = support::find_by_field(jobs, "/job_id", "memory-evolution-preference-001")?;
	let core_attachment =
		support::find_by_field(jobs, "/job_id", "core-archival-core-block-attachment-001")?;
	let archival_fallback =
		support::find_by_field(jobs, "/job_id", "core-archival-archival-fallback-001")?;

	assert_eq!(preference.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		preference.pointer("/evolution/history_readback_encoded").and_then(Value::as_bool),
		Some(true)
	);
	assert!(support::array_contains_str(preference, "/evolution/history_event_types", "update")?);
	assert_eq!(core_attachment.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(archival_fallback.pointer("/status").and_then(Value::as_str), Some("pass"));

	let adapters = support::array_at(&report, "/external_adapters/adapters")?;
	let mem0 = support::find_by_field(adapters, "/adapter_id", "mem0_openmemory_live_baseline")?;
	let letta = support::find_by_field(adapters, "/adapter_id", "letta_research_gate")?;
	let mem0_scenarios = support::array_at(mem0, "/scenarios")?;
	let mem0_history =
		support::find_by_field(mem0_scenarios, "/scenario_id", "preference_correction_history")?;
	let mem0_entity =
		support::find_by_field(mem0_scenarios, "/scenario_id", "entity_scoped_personalization")?;

	assert_eq!(mem0_history.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0_entity.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(mem0_history.pointer("/comparison_outcome").and_then(Value::as_str), Some("loss"));
	assert_eq!(mem0_entity.pointer("/comparison_outcome").and_then(Value::as_str), Some("tie"));

	let letta_scenarios = support::array_at(letta, "/scenarios")?;
	let letta_core =
		support::find_by_field(letta_scenarios, "/scenario_id", "core_block_attachment_readback")?;
	let letta_fallback =
		support::find_by_field(letta_scenarios, "/scenario_id", "archival_fallback_readback")?;

	for scenario in [letta_core, letta_fallback] {
		assert_eq!(
			scenario.pointer("/suite_id").and_then(Value::as_str),
			Some("core_archival_memory")
		);
		assert_eq!(scenario.pointer("/status").and_then(Value::as_str), Some("blocked"));
		assert_eq!(
			scenario.pointer("/comparison_outcome").and_then(Value::as_str),
			Some("blocked")
		);
	}

	Ok(())
}

#[test]
fn context_trajectory_fixtures_report_blocked_openviking_gates() -> Result<()> {
	let report = support::run_json_report_from(support::context_trajectory_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(3)
	);

	let suites = support::array_at(&report, "/suites")?;
	let context = support::find_by_field(suites, "/suite_id", "context_trajectory")?;

	assert_eq!(context.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(context.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = support::array_at(&report, "/jobs")?;
	let staged = support::find_by_field(
		jobs,
		"/job_id",
		"context-trajectory-openviking-staged-retrieval-001",
	)?;
	let hierarchy = support::find_by_field(
		jobs,
		"/job_id",
		"context-trajectory-openviking-hierarchy-selection-001",
	)?;
	let recursive = support::find_by_field(
		jobs,
		"/job_id",
		"context-trajectory-openviking-recursive-expansion-001",
	)?;

	assert_eq!(staged.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(hierarchy.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(recursive.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		staged.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("openviking.stage_artifact_gate")
	);
	assert_eq!(
		hierarchy.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("openviking.hierarchy_artifact_gate")
	);
	assert_eq!(
		recursive.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("openviking.recursive_expansion_gate")
	);

	let staged_stages = support::array_at(staged, "/trace_explainability/stages")?;
	let staged_gate =
		support::find_by_field(staged_stages, "/stage_name", "openviking.stage_artifact_gate")?;

	assert!(support::array_contains_str(staged_gate, "/dropped_evidence", "trajectory-win-decoy")?);

	let hierarchy_stages = support::array_at(hierarchy, "/trace_explainability/stages")?;
	let hierarchy_gate = support::find_by_field(
		hierarchy_stages,
		"/stage_name",
		"openviking.hierarchy_artifact_gate",
	)?;

	assert!(support::array_contains_str(
		hierarchy_gate,
		"/dropped_evidence",
		"hierarchy-design-win-decoy"
	)?);

	let recursive_stages = support::array_at(recursive, "/trace_explainability/stages")?;
	let recursive_gate = support::find_by_field(
		recursive_stages,
		"/stage_name",
		"openviking.recursive_expansion_gate",
	)?;

	assert!(support::array_contains_str(
		recursive_gate,
		"/dropped_evidence",
		"recursive-expansion-win-decoy"
	)?);
	assert!(
		staged.pointer("/reason").and_then(Value::as_str).is_some_and(
			|reason| reason.contains("same-corpus output returns expected evidence ids")
		)
	);

	Ok(())
}
