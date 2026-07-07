use serde_json::Value;

pub(crate) fn assert_root_summary_counts(report: &Value) {
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(84));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(20));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(77));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/expected_evidence_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/irrelevant_context_ratio").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(report.pointer("/summary/stale_retrieval_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(11)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(16)
	);
	assert_eq!(
		report.pointer("/summary/temporal_validity_not_encoded_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/scope_check_count").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/scope_correct_count").and_then(Value::as_u64), Some(4));
	assert_eq!(report.pointer("/summary/scope_violation_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_case_count").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_pass_count").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report.pointer("/summary/evidence_required_count").and_then(Value::as_u64),
		Some(185)
	);
	assert_eq!(
		report.pointer("/summary/evidence_covered_count").and_then(Value::as_u64),
		Some(185)
	);
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/trace_explainability_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/summary/wrong_result_stage_attribution_count").and_then(Value::as_u64),
		Some(0)
	);
}

pub(crate) fn assert_root_consolidation_summary(report: &Value) {
	assert_eq!(
		report.pointer("/summary/consolidation/proposal_count").and_then(Value::as_u64),
		Some(6)
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
		report.pointer("/summary/memory_summary/job_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/invalid_top_of_mind_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/memory_summary/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
}

pub(crate) fn assert_root_local_organizer_summary(report: &Value) {
	assert_eq!(
		report.pointer("/summary/local_organizer/job_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/summary/local_organizer/extraction_f1_not_encoded_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report.pointer("/summary/local_organizer/json_schema_valid_count").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/summary/local_organizer/citation_source_ref_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		report.pointer("/summary/local_organizer/unsupported_claim_rate").and_then(Value::as_f64),
		Some(0.0)
	);
	assert_eq!(
		report.pointer("/summary/local_organizer/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report
			.pointer("/summary/local_organizer/silent_memory_authority_mutation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/summary/local_organizer/escalation_rate").and_then(Value::as_f64),
		Some(1.0)
	);
}
