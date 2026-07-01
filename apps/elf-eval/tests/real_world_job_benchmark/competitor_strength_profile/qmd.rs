use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_qmd_strength_profile(report: &Value) -> Result<()> {
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

pub(crate) fn assert_qmd_wrong_result_diagnosis(report: &Value) -> Result<()> {
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
