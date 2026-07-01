use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn adversarial_quality_fixtures_score_scoreboard_gates() -> Result<()> {
	let report = support::run_json_report_from(support::adversarial_quality_fixture_dir())?;

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

	let result_states = support::string_array_at(&report, "/scoreboard/result_states")?;
	let evidence_classes = support::string_array_at(&report, "/scoreboard/evidence_classes")?;

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
		support::string_array_at(&report, "/scoreboard/job_typed_non_pass_states_present")?,
		Vec::<String>::new()
	);

	for state in ["blocked", "incomplete", "not_encoded", "not_tested", "wrong_result"] {
		assert!(support::array_contains_str(
			&report,
			"/scoreboard/typed_non_pass_states_present",
			state
		)?);
		assert!(support::array_contains_str(
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

	let suites = support::array_at(&report, "/suites")?;
	let adversarial = support::find_by_field(suites, "/suite_id", "adversarial_quality")?;

	assert_eq!(adversarial.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(adversarial.pointer("/encoded_job_count").and_then(Value::as_u64), Some(5));

	Ok(())
}

fn assert_scoreboard_rows_expose_quantitative_and_blocker_contract(report: &Value) -> Result<()> {
	let rows = support::array_at(report, "/scoreboard/rows")?;
	let elf = support::find_by_field(rows, "/product_id", "elf_current_report")?;
	let qmd = support::find_by_field(rows, "/product_id", "qmd")?;
	let pageindex = support::find_by_field(rows, "/product_id", "vectifyai_pageindex")?;
	let openkb = support::find_by_field(rows, "/product_id", "vectifyai_openkb")?;
	let honcho = support::find_by_field(rows, "/product_id", "plastic_labs_honcho")?;

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
	assert!(support::array_contains_str(
		elf,
		"/next_evidence",
		"Run a Docker-contained product-runtime adapter for this row."
	)?);
	assert!(support::array_contains_str(
		elf,
		"/next_evidence",
		"Record container image digest evidence."
	)?);
	assert_eq!(qmd.pointer("/product_name").and_then(Value::as_str), Some("qmd"));
	assert_eq!(qmd.pointer("/evidence_class").and_then(Value::as_str), Some("live_real_world"));
	assert_eq!(qmd.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(qmd.pointer("/product_runtime").and_then(Value::as_bool), Some(true));
	assert_eq!(qmd.pointer("/container_digest_identified").and_then(Value::as_bool), Some(false));
	assert!(qmd.pointer("/metrics/retrieval/recall_at_k").is_some_and(Value::is_null));
	assert!(support::array_contains_str(
		qmd,
		"/next_evidence",
		"Record container image digest evidence."
	)?);

	crate::assert_tracked_external_blocker_row(pageindex, "VectifyAI PageIndex", true)?;
	crate::assert_tracked_external_blocker_row(openkb, "VectifyAI OpenKB", true)?;
	crate::assert_tracked_external_blocker_row(honcho, "plastic-labs Honcho", false)?;

	Ok(())
}
