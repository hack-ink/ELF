use std::fs;

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn source_backed_quality_report_emits_xy1155_metrics_and_scenarios() -> Result<()> {
	let report = support::run_json_report_from(support::real_world_memory_fixture_dir())?;
	let quality = report
		.pointer("/source_backed_quality")
		.ok_or_else(|| eyre::eyre!("missing source_backed_quality report"))?;

	assert_eq!(
		quality.pointer("/schema").and_then(Value::as_str),
		Some("elf.source_backed_memory_quality_benchmark/v1")
	);
	assert_eq!(quality.pointer("/hard_fail_passed").and_then(Value::as_bool), Some(true));
	assert_eq!(quality.pointer("/metrics/cross_scope_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(
		quality.pointer("/metrics/journal_only_authority_claim_count").and_then(Value::as_u64),
		Some(0)
	);

	let expected_metrics = [
		"expected_evidence_recall",
		"precision_at_5",
		"irrelevant_context_ratio",
		"source_ref_coverage",
		"stale_suppression_rate",
		"correction_persistence_rate",
		"delete_tombstone_suppression_rate",
		"unsupported_claim_rate",
		"cross_scope_leak_count",
		"journal_only_authority_claim_count",
		"context_pack_activation_precision",
		"context_pack_activation_recall",
		"activation_trace_coverage",
		"mean_latency_ms",
		"total_cost",
	];

	assert_eq!(
		quality.pointer("/required_metric_names"),
		Some(&serde_json::json!(expected_metrics))
	);

	for metric in expected_metrics {
		assert!(
			quality.pointer(&format!("/metrics/{metric}")).is_some(),
			"missing source-backed quality metric {metric}"
		);
	}

	assert_eq!(
		quality.pointer("/metrics/context_pack_activation_precision").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		quality.pointer("/metrics/context_pack_activation_recall").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		quality.pointer("/metrics/activation_trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);

	assert_context_pack_decisions(quality);

	let scenarios = support::array_at(quality, "/scenario_coverage")?;

	assert_required_scenarios_pass(quality, scenarios)?;

	Ok(())
}

#[test]
fn source_backed_quality_task_is_registered() -> Result<()> {
	let makefile =
		fs::read_to_string(support::workspace_root()?.join("makefiles/benchmark-memory-b.toml"))?;

	for task in [
		"[tasks.source-backed-memory-quality]",
		"[tasks.source-backed-memory-quality-json]",
		"[tasks.source-backed-memory-quality-validate]",
		"[tasks.source-backed-memory-quality-report]",
	] {
		assert!(makefile.contains(task), "missing cargo make task {task}");
	}

	Ok(())
}

fn assert_context_pack_decisions(quality: &Value) {
	for (field, expected) in [
		("expected_enabled", 2),
		("expected_suppressed", 1),
		("expected_disabled", 1),
		("expected_stale_suppressed", 1),
		("expected_blocked", 1),
		("expected_pinned_ineligible", 1),
		("incorrect_decisions", 0),
	] {
		assert_eq!(
			quality.pointer(&format!("/context_pack_decisions/{field}")).and_then(Value::as_u64),
			Some(expected),
			"context pack decision count mismatch for {field}"
		);
	}
}

fn assert_required_scenarios_pass(quality: &Value, scenarios: &[Value]) -> Result<()> {
	let missing = scenarios
		.iter()
		.filter(|scenario| {
			scenario.pointer("/status").and_then(Value::as_str) == Some("not_encoded")
		})
		.map(|scenario| {
			scenario
				.pointer("/scenario")
				.and_then(Value::as_str)
				.unwrap_or("<missing scenario>")
				.to_string()
		})
		.collect::<Vec<_>>();

	assert!(missing.is_empty(), "required scenarios are not encoded: {missing:?}");

	for scenario in scenarios {
		assert_eq!(
			scenario.pointer("/status").and_then(Value::as_str),
			Some("pass"),
			"required scenario did not pass: {scenario:?}"
		);
	}
	for scenario in [
		"context_pack_relevant_auto_activation",
		"context_pack_irrelevant_suppression",
		"context_pack_disabled_suppression",
		"context_pack_stale_suppression",
		"journal_only_current_fact_trap",
		"dreaming_no_silent_mutation",
		"authoritative_revalidation",
		"recall_debug_privacy",
	] {
		assert_scenario_passes(quality, scenario)?;
	}

	Ok(())
}

fn assert_scenario_passes(report: &Value, scenario_id: &str) -> Result<()> {
	let scenarios = support::array_at(report, "/scenario_coverage")?;
	let scenario = scenarios
		.iter()
		.find(|scenario| scenario.pointer("/scenario").and_then(Value::as_str) == Some(scenario_id))
		.ok_or_else(|| eyre::eyre!("missing scenario {scenario_id}"))?;

	assert_eq!(
		scenario.pointer("/status").and_then(Value::as_str),
		Some("pass"),
		"scenario {scenario_id} did not pass"
	);

	Ok(())
}
