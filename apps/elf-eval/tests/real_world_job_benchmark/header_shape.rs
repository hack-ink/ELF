use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

pub(super) fn assert_dreaming_readiness_ledger_header(ledger: &Value) -> Result<()> {
	assert_eq!(
		ledger.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_readiness_stage_ledger/v1")
	);
	assert_eq!(ledger.pointer("/authority").and_then(Value::as_str), Some("XY-951"));

	for term in ["improved", "regressed", "unchanged", "blocked", "not_tested"] {
		assert!(support::array_contains_str(ledger, "/judgment_terms", term)?);
	}
	for term in ["pass", "wrong_result", "blocked", "not_tested", "not_encoded"] {
		assert!(support::array_contains_str(ledger, "/count_fields", term)?);
	}

	Ok(())
}

pub(super) fn assert_dreaming_readiness_stage_shape(
	ledger: &Value,
	stages: &[Value],
) -> Result<()> {
	assert_eq!(stages.len(), 8);

	for stage_id in [
		"current_vs_historical_correctness",
		"preference_evolution",
		"deletion_ttl_tombstone_behavior",
		"reviewable_consolidation",
		"memory_summary_top_of_mind_behavior",
		"proactive_brief_readiness",
		"scheduled_memory_task_readiness",
		"final_competitor_retest_status",
	] {
		support::find_by_field(stages, "/stage_id", stage_id)?;
	}
	for stage in stages {
		let stage_id =
			stage.pointer("/stage_id").and_then(Value::as_str).unwrap_or("<missing stage_id>");

		assert!(
			!support::array_at(stage, "/baseline_commands")?.is_empty(),
			"{stage_id} missing baseline commands"
		);
		assert!(
			!support::array_at(stage, "/post_stage_commands")?.is_empty(),
			"{stage_id} missing post-stage commands"
		);
		assert!(
			!support::array_at(stage, "/evidence_files")?.is_empty(),
			"{stage_id} missing evidence files"
		);

		for count_field in support::string_array_at(ledger, "/count_fields")? {
			let pointer = format!("/baseline_counts/{count_field}");

			assert!(
				stage.pointer(&pointer).and_then(Value::as_u64).is_some(),
				"{stage_id} missing {pointer}"
			);
		}

		let judgment = stage
			.pointer("/comparison_judgment")
			.and_then(Value::as_str)
			.ok_or_else(|| eyre::eyre!("{stage_id} missing comparison_judgment"))?;

		assert!(support::array_contains_str(ledger, "/judgment_terms", judgment)?);
	}

	Ok(())
}
