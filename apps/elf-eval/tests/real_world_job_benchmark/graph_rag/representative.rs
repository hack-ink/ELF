use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn graph_rag_representative_fixtures_report_typed_non_pass_states() -> Result<()> {
	let report = support::run_json_report_from(support::graph_rag_external_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(
		report.pointer("/summary/knowledge/citation_coverage").and_then(Value::as_f64),
		Some(0.667)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(
		report.pointer("/summary/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(1)
	);

	let jobs = support::array_at(&report, "/jobs")?;
	let ragflow =
		support::find_by_field(jobs, "/job_id", "graph-rag-ragflow-reference-chunks-001")?;
	let lightrag =
		support::find_by_field(jobs, "/job_id", "graph-rag-lightrag-context-sources-001")?;
	let graphrag = support::find_by_field(jobs, "/job_id", "graph-rag-graphrag-output-tables-001")?;
	let graphiti =
		support::find_by_field(jobs, "/job_id", "graph-rag-graphiti-temporal-validity-001")?;
	let graphify = support::find_by_field(jobs, "/job_id", "graph-rag-graphify-graph-report-001")?;

	assert_eq!(ragflow.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(lightrag.pointer("/status").and_then(Value::as_str), Some("incomplete"));
	assert_eq!(graphrag.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphiti.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(graphify.pointer("/status").and_then(Value::as_str), Some("wrong_result"));
	assert_eq!(
		graphify.pointer("/knowledge/stale_claim_detection").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(
		graphify.pointer("/knowledge/unsupported_summary_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		graphiti.pointer("/evolution/temporal_validity_not_encoded").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		graphiti.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("graphiti.provider_boundary")
	);
	assert!(support::array_contains_str(
		graphiti,
		"/produced_evidence",
		"graphiti-current-fact-contract"
	)?);
	assert!(support::array_contains_str(
		graphiti,
		"/produced_evidence",
		"graphiti-historical-fact-contract"
	)?);
	assert!(support::array_contains_str(
		graphiti,
		"/produced_evidence",
		"graphiti-provider-boundary"
	)?);
	assert!(support::array_contains_str(
		graphify,
		"/produced_evidence",
		"graphify-source-location-output"
	)?);

	Ok(())
}
