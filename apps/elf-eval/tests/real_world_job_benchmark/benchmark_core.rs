use super::*;

#[test]
fn smoke_fixture_produces_typed_json_report() -> Result<()> {
	let report = run_json_report()?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.real_world_job_report/v1")
	);
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/unsupported_claim_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/wrong_result_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/external_adapters/summary/adapter_count").and_then(Value::as_u64),
		Some(26)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/live_real_world_count").and_then(Value::as_u64),
		Some(5)
	);
	assert_eq!(
		report.pointer("/external_adapters/summary/research_gate_count").and_then(Value::as_u64),
		Some(14)
	);

	let jobs = array_at(&report, "/jobs")?;
	let job = find_by_field(jobs, "/job_id", "work-resume-stale-worktree-001")?;

	assert_eq!(job.pointer("/suite_id").and_then(Value::as_str), Some("work_resume"));
	assert_eq!(job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(job.pointer("/latency_ms").and_then(Value::as_f64), Some(2.0));
	assert_eq!(job.pointer("/cost/amount").and_then(Value::as_f64), Some(0.0));

	let expected_evidence = array_at(job, "/expected_evidence")?;
	let produced_evidence = array_at(job, "/produced_evidence")?;

	assert_eq!(expected_evidence.len(), 2);
	assert_eq!(produced_evidence.len(), 1);
	assert_eq!(produced_evidence.first().and_then(Value::as_str), Some("xy844-current-worktree"));

	let suites = array_at(&report, "/suites")?;
	let encoded_suite = find_by_field(suites, "/suite_id", "work_resume")?;
	let capture_suite = find_by_field(suites, "/suite_id", "capture_integration")?;
	let unencoded_suite = find_by_field(suites, "/suite_id", "retrieval")?;

	assert_eq!(encoded_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(encoded_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(capture_suite.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(capture_suite.pointer("/encoded_job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(unencoded_suite.pointer("/status").and_then(Value::as_str), Some("not_encoded"));

	let capture_fixture_backed = array_at(&report, "/capture_integration/fixture_backed")?;

	assert!(capture_fixture_backed.iter().any(|value| {
		value.as_str().is_some_and(|item| item.contains("agentmemory-style hook capture"))
	}));

	let capture_not_encoded = array_at(&report, "/capture_integration/not_encoded")?;

	assert!(capture_not_encoded.iter().any(|value| {
		value.as_str().is_some_and(|item| item.contains("No live external hook ingestion"))
	}));

	Ok(())
}

#[test]
fn capture_integration_fixtures_score_redaction_and_source_ids() -> Result<()> {
	let report = run_json_report_from(capture_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(3));
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));

	let suites = array_at(&report, "/suites")?;
	let capture = find_by_field(suites, "/suite_id", "capture_integration")?;

	assert_eq!(capture.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(capture.pointer("/encoded_job_count").and_then(Value::as_u64), Some(3));

	let jobs = array_at(&report, "/jobs")?;
	let source_id = find_by_field(jobs, "/job_id", "capture-source-id-binding-001")?;
	let redaction = find_by_field(jobs, "/job_id", "capture-write-policy-redaction-001")?;

	assert!(array_contains_str(source_id, "/produced_evidence", "source-id-release-summary")?);
	assert!(array_contains_str(source_id, "/produced_evidence", "source-id-command-log")?);
	assert_eq!(redaction.pointer("/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert!(
		redaction
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| !answer.contains("orchid-envelope"))
	);

	Ok(())
}

#[test]
fn source_library_fixtures_score_saved_sources_without_memory_promotion() -> Result<()> {
	let report = run_json_report_from(source_library_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/source_ref_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(report.pointer("/summary/quote_coverage").and_then(Value::as_f64), Some(1.0));

	let suites = array_at(&report, "/suites")?;
	let source_library = find_by_field(suites, "/suite_id", "source_library")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/encoded_job_count").and_then(Value::as_u64), Some(2));

	let jobs = array_at(&report, "/jobs")?;
	let long_doc = find_by_field(jobs, "/job_id", "source-library-long-doc-001")?;
	let thread = find_by_field(jobs, "/job_id", "source-library-social-thread-001")?;

	assert!(array_contains_str(long_doc, "/produced_evidence", "article-source-record")?);
	assert!(array_contains_str(long_doc, "/produced_evidence", "article-hydrated-excerpt")?);
	assert!(array_contains_str(thread, "/produced_evidence", "thread-source-record")?);
	assert!(array_contains_str(thread, "/produced_evidence", "thread-promotion-boundary")?);
	assert!(long_doc.pointer("/produced_answer").and_then(Value::as_str).is_some_and(|answer| {
		answer.contains("does not automatically create a durable Memory Note")
	}));
	assert!(
		thread
			.pointer("/produced_answer")
			.and_then(Value::as_str)
			.is_some_and(|answer| answer.contains("explicit add_note or reviewed promotion"))
	);

	Ok(())
}

#[test]
fn adversarial_quality_fixtures_score_scoreboard_gates() -> Result<()> {
	let report = run_json_report_from(adversarial_quality_fixture_dir())?;

	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/encoded_suite_count").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(5));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/unsupported_claim").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/stale_answer_count").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		report.pointer("/summary/conflict_detection_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/summary/update_rationale_available_count").and_then(Value::as_u64),
		Some(3)
	);
	assert_eq!(
		report.pointer("/summary/history_readback_encoded_count").and_then(Value::as_u64),
		Some(1)
	);

	let result_states = string_array_at(&report, "/scoreboard/result_states")?;
	let evidence_classes = string_array_at(&report, "/scoreboard/evidence_classes")?;

	assert_eq!(
		result_states,
		[
			"pass",
			"wrong_result",
			"incomplete",
			"blocked",
			"not_tested",
			"not_encoded",
			"not_comparable",
			"unsupported_claim",
		]
		.map(str::to_owned)
	);
	assert_eq!(
		evidence_classes,
		["fixture_backed", "live_baseline", "live_real_world", "research_gate"].map(str::to_owned)
	);
	assert_eq!(
		report.pointer("/scoreboard/summary_claim").and_then(Value::as_str),
		Some("typed_non_pass_present")
	);
	assert_eq!(
		report.pointer("/scoreboard/job_summary_claim").and_then(Value::as_str),
		Some("all_encoded_jobs_passed")
	);
	assert_eq!(
		report.pointer("/scoreboard/job_typed_non_pass_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		report.pointer("/scoreboard/external_adapter_typed_non_pass_count").and_then(Value::as_u64),
		Some(240)
	);
	assert_eq!(
		report.pointer("/scoreboard/typed_non_pass_count").and_then(Value::as_u64),
		Some(240)
	);
	assert_eq!(
		string_array_at(&report, "/scoreboard/job_typed_non_pass_states_present")?,
		Vec::<String>::new()
	);

	for state in ["blocked", "incomplete", "not_encoded", "not_tested", "wrong_result"] {
		assert!(array_contains_str(&report, "/scoreboard/typed_non_pass_states_present", state)?);
		assert!(array_contains_str(
			&report,
			"/scoreboard/external_adapter_typed_non_pass_states_present",
			state
		)?);
	}

	assert_eq!(
		report.pointer("/scoreboard/unqualified_win_claim_allowed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report.pointer("/scoreboard/evidence_class_counts/live_baseline").and_then(Value::as_u64),
		Some(6)
	);
	assert_eq!(
		report.pointer("/scoreboard/metric_basis").and_then(Value::as_str),
		Some("produced_evidence_order")
	);
	assert_eq!(report.pointer("/scoreboard/retrieval_k").and_then(Value::as_u64), Some(5));

	assert_scoreboard_rows_expose_quantitative_and_blocker_contract(&report)?;

	let suites = array_at(&report, "/suites")?;
	let adversarial = find_by_field(suites, "/suite_id", "adversarial_quality")?;

	assert_eq!(adversarial.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(adversarial.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	Ok(())
}

fn assert_scoreboard_rows_expose_quantitative_and_blocker_contract(report: &Value) -> Result<()> {
	let rows = array_at(report, "/scoreboard/rows")?;
	let elf = find_by_field(rows, "/product_id", "elf_current_report")?;
	let qmd = find_by_field(rows, "/product_id", "qmd")?;
	let pageindex = find_by_field(rows, "/product_id", "vectifyai_pageindex")?;
	let openkb = find_by_field(rows, "/product_id", "vectifyai_openkb")?;
	let honcho = find_by_field(rows, "/product_id", "plastic_labs_honcho")?;

	assert_eq!(rows.len(), 20);
	assert_eq!(elf.pointer("/product_name").and_then(Value::as_str), Some("ELF"));
	assert_eq!(elf.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
	assert_eq!(elf.pointer("/result_state").and_then(Value::as_str), Some("not_comparable"));
	assert_eq!(elf.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/same_corpus").and_then(Value::as_bool), Some(true));
	assert_eq!(elf.pointer("/source_id_mapped").and_then(Value::as_bool), Some(true));
	assert_eq!(elf.pointer("/held_out").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/leakage_audited").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/product_runtime").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/container_digest_identified").and_then(Value::as_bool), Some(false));
	assert_eq!(
		elf.pointer("/metrics/retrieval/metric_basis").and_then(Value::as_str),
		Some("produced_evidence_order")
	);
	assert_eq!(elf.pointer("/metrics/retrieval/k").and_then(Value::as_u64), Some(5));
	assert!(elf.pointer("/metrics/retrieval/recall_at_k").and_then(Value::as_f64).is_some());
	assert!(elf.pointer("/metrics/retrieval/precision_at_k").and_then(Value::as_f64).is_some());
	assert!(elf.pointer("/metrics/retrieval/mrr").and_then(Value::as_f64).is_some());
	assert!(elf.pointer("/metrics/retrieval/ndcg").and_then(Value::as_f64).is_some());
	assert_eq!(
		elf.pointer("/metrics/lifecycle/stale_suppression").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		elf.pointer("/metrics/coverage/source_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert!(array_contains_str(
		elf,
		"/next_evidence",
		"Run a Docker-contained product-runtime adapter for this row."
	)?);
	assert!(array_contains_str(elf, "/next_evidence", "Record container image digest evidence.")?);
	assert_eq!(qmd.pointer("/product_name").and_then(Value::as_str), Some("qmd"));
	assert_eq!(qmd.pointer("/evidence_class").and_then(Value::as_str), Some("live_real_world"));
	assert_eq!(qmd.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(qmd.pointer("/product_runtime").and_then(Value::as_bool), Some(true));
	assert_eq!(qmd.pointer("/container_digest_identified").and_then(Value::as_bool), Some(false));
	assert!(qmd.pointer("/metrics/retrieval/recall_at_k").is_some_and(Value::is_null));
	assert!(array_contains_str(qmd, "/next_evidence", "Record container image digest evidence.")?);

	assert_tracked_external_blocker_row(pageindex, "VectifyAI PageIndex", true)?;
	assert_tracked_external_blocker_row(openkb, "VectifyAI OpenKB", true)?;
	assert_tracked_external_blocker_row(honcho, "plastic-labs Honcho", false)?;

	Ok(())
}

pub(super) fn assert_tracked_external_blocker_row(
	row: &Value,
	product_name: &str,
	same_corpus: bool,
) -> Result<()> {
	assert_eq!(row.pointer("/product_name").and_then(Value::as_str), Some(product_name));
	assert_eq!(row.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert_eq!(row.pointer("/evidence_class").and_then(Value::as_str), Some("research_gate"));
	assert_eq!(row.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/same_corpus").and_then(Value::as_bool), Some(same_corpus));
	assert_eq!(row.pointer("/source_id_mapped").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/held_out").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/leakage_audited").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/product_runtime").and_then(Value::as_bool), Some(false));
	assert_eq!(row.pointer("/container_digest_identified").and_then(Value::as_bool), Some(false));
	assert!(row.pointer("/metrics/retrieval/recall_at_k").is_some_and(Value::is_null));
	assert!(row.pointer("/metrics/retrieval/precision_at_k").is_some_and(Value::is_null));
	assert!(row.pointer("/metrics/retrieval/mrr").is_some_and(Value::is_null));
	assert!(row.pointer("/metrics/retrieval/ndcg").is_some_and(Value::is_null));
	assert!(array_contains_str(
		row,
		"/next_evidence",
		"Map returned evidence to stable source ids."
	)?);
	assert!(array_contains_str(
		row,
		"/next_evidence",
		"Run a Docker-contained product-runtime adapter for this row."
	)?);
	assert!(array_contains_str(row, "/next_evidence", "Record container image digest evidence.")?);

	if same_corpus {
		assert!(!array_contains_str(
			row,
			"/next_evidence",
			"Map this product to the same corpus."
		)?);
	} else {
		assert!(array_contains_str(row, "/next_evidence", "Map this product to the same corpus.")?);
	}

	Ok(())
}
