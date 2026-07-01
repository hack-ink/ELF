use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_root_scoreboard_summary(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/scoreboard/summary_claim").and_then(Value::as_str),
		Some("typed_non_pass_present")
	);
	assert_eq!(
		report.pointer("/scoreboard/job_summary_claim").and_then(Value::as_str),
		Some("typed_non_pass_present")
	);
	assert_eq!(
		report.pointer("/scoreboard/job_typed_non_pass_count").and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report.pointer("/scoreboard/external_adapter_typed_non_pass_count").and_then(Value::as_u64),
		Some(240)
	);
	assert_eq!(
		report.pointer("/scoreboard/typed_non_pass_count").and_then(Value::as_u64),
		Some(247)
	);
	assert_eq!(
		report.pointer("/scoreboard/unqualified_win_claim_allowed").and_then(Value::as_bool),
		Some(false)
	);
	assert!(support::array_contains_str(report, "/scoreboard/result_states", "not_comparable")?);
	assert_eq!(
		report.pointer("/scoreboard/metric_basis").and_then(Value::as_str),
		Some("produced_evidence_order")
	);
	assert_eq!(report.pointer("/scoreboard/retrieval_k").and_then(Value::as_u64), Some(5));

	assert_root_scoreboard_rows(report)?;

	for state in ["blocked", "incomplete", "not_encoded", "not_tested", "wrong_result"] {
		assert!(support::array_contains_str(
			report,
			"/scoreboard/typed_non_pass_states_present",
			state
		)?);
	}

	assert_eq!(
		support::string_array_at(report, "/scoreboard/job_typed_non_pass_states_present")?,
		["blocked"].map(str::to_owned)
	);

	for state in ["blocked", "incomplete", "not_encoded", "not_tested", "wrong_result"] {
		assert!(support::array_contains_str(
			report,
			"/scoreboard/external_adapter_typed_non_pass_states_present",
			state
		)?);
	}

	Ok(())
}

fn assert_root_scoreboard_rows(report: &Value) -> Result<()> {
	let rows = support::array_at(report, "/scoreboard/rows")?;
	let elf = support::find_by_field(rows, "/product_id", "elf_current_report")?;
	let qmd = support::find_by_field(rows, "/product_id", "qmd")?;
	let graphify = support::find_by_field(rows, "/product_id", "graphify")?;
	let pageindex = support::find_by_field(rows, "/product_id", "vectifyai_pageindex")?;
	let openkb = support::find_by_field(rows, "/product_id", "vectifyai_openkb")?;
	let honcho = support::find_by_field(rows, "/product_id", "plastic_labs_honcho")?;

	assert_eq!(rows.len(), 20);
	assert_eq!(elf.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert_eq!(elf.pointer("/evidence_class").and_then(Value::as_str), Some("fixture_backed"));
	assert_eq!(elf.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/same_corpus").and_then(Value::as_bool), Some(true));
	assert_eq!(elf.pointer("/source_id_mapped").and_then(Value::as_bool), Some(true));
	assert_eq!(elf.pointer("/product_runtime").and_then(Value::as_bool), Some(false));
	assert_eq!(elf.pointer("/metrics/retrieval/recall_at_k").and_then(Value::as_f64), Some(0.988));
	assert_eq!(
		elf.pointer("/metrics/retrieval/precision_at_k").and_then(Value::as_f64),
		Some(0.415)
	);
	assert_eq!(elf.pointer("/metrics/retrieval/mrr").and_then(Value::as_f64), Some(0.988));
	assert_eq!(elf.pointer("/metrics/retrieval/ndcg").and_then(Value::as_f64), Some(0.985));
	assert_eq!(
		elf.pointer("/metrics/lifecycle/stale_suppression").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		elf.pointer("/metrics/lifecycle/update_correctness").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		elf.pointer("/metrics/lifecycle/delete_correctness").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		elf.pointer("/metrics/coverage/typed_non_pass_count").and_then(Value::as_u64),
		Some(7)
	);
	assert!(support::array_contains_str(
		elf,
		"/next_evidence",
		"Run a Docker-contained product-runtime adapter for this row."
	)?);

	for competitor in [qmd, graphify] {
		assert_eq!(
			competitor.pointer("/evidence_class").and_then(Value::as_str),
			Some("live_real_world")
		);
		assert_eq!(
			competitor.pointer("/result_state").and_then(Value::as_str),
			Some("wrong_result")
		);
		assert_eq!(competitor.pointer("/product_runtime").and_then(Value::as_bool), Some(true));
		assert_eq!(
			competitor.pointer("/container_digest_identified").and_then(Value::as_bool),
			Some(false)
		);
		assert!(competitor.pointer("/metrics/retrieval/recall_at_k").is_some_and(Value::is_null));
		assert!(support::array_contains_str(
			competitor,
			"/next_evidence",
			"Record container image digest evidence."
		)?);
	}

	crate::assert_tracked_external_blocker_row(pageindex, "VectifyAI PageIndex", true)?;
	crate::assert_tracked_external_blocker_row(openkb, "VectifyAI OpenKB", true)?;
	crate::assert_tracked_external_blocker_row(honcho, "plastic-labs Honcho", false)?;

	Ok(())
}
