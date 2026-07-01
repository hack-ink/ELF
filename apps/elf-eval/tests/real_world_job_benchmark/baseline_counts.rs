use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_dreaming_readiness_baseline_counts(
	ledger: &Value,
	stages: &[Value],
) -> Result<()> {
	let current = support::find_by_field(stages, "/stage_id", "current_vs_historical_correctness")?;

	assert_eq!(current.pointer("/baseline_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(current.pointer("/baseline_counts/wrong_result").and_then(Value::as_u64), Some(5));
	assert_eq!(current.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(current.pointer("/post_stage_counts/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(current.pointer("/comparison_judgment").and_then(Value::as_str), Some("improved"));
	assert!(
		current
			.pointer("/baseline_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("five current-vs-historical jobs"))
	);
	assert!(
		current
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("passes all six encoded jobs"))
	);

	let preference = support::find_by_field(stages, "/stage_id", "preference_evolution")?;

	assert_eq!(
		preference.pointer("/baseline_counts/wrong_result").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(preference.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(
		preference.pointer("/post_stage_counts/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		preference.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);

	let tombstone = support::find_by_field(stages, "/stage_id", "deletion_ttl_tombstone_behavior")?;

	assert_eq!(tombstone.pointer("/baseline_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(tombstone.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(1));
	assert_eq!(
		tombstone.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("unchanged")
	);
	assert!(
		tombstone
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("tombstone and invalidation evidence"))
	);

	let consolidation = support::find_by_field(stages, "/stage_id", "reviewable_consolidation")?;

	assert_eq!(
		consolidation.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(
		consolidation.pointer("/baseline_counts/not_encoded").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(consolidation.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(
		consolidation.pointer("/post_stage_counts/not_encoded").and_then(Value::as_u64),
		Some(0)
	);
	assert!(
		consolidation
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("apply/defer/discard audit")
				&& basis.contains("zero source mutations"))
	);

	let scheduled = support::find_by_field(stages, "/stage_id", "scheduled_memory_task_readiness")?;

	assert_eq!(scheduled.pointer("/comparison_judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(scheduled.pointer("/baseline_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(scheduled.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(scheduled.pointer("/post_stage_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(
		scheduled.pointer("/post_stage_counts/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		scheduled.pointer("/post_stage_counts/source_mutation_count").and_then(Value::as_u64),
		Some(0)
	);

	assert_dreaming_final_competitor_retest_stage(ledger, stages)?;
	assert_dreaming_memory_summary_stage(stages)?;
	assert_dreaming_proactive_brief_stage(stages)?;

	Ok(())
}

fn assert_dreaming_final_competitor_retest_stage(ledger: &Value, stages: &[Value]) -> Result<()> {
	let retest = support::find_by_field(stages, "/stage_id", "final_competitor_retest_status")?;

	assert_eq!(retest.pointer("/baseline_counts/pass").and_then(Value::as_u64), Some(22));
	assert_eq!(retest.pointer("/baseline_counts/wrong_result").and_then(Value::as_u64), Some(5));
	assert_eq!(retest.pointer("/baseline_counts/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(retest.pointer("/baseline_counts/not_tested").and_then(Value::as_u64), Some(11));
	assert_eq!(retest.pointer("/baseline_counts/not_encoded").and_then(Value::as_u64), Some(11));
	assert_eq!(retest.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(40));
	assert_eq!(retest.pointer("/post_stage_counts/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(retest.pointer("/post_stage_counts/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(retest.pointer("/post_stage_counts/not_encoded").and_then(Value::as_u64), Some(19));
	assert_eq!(retest.pointer("/qmd_post_stage_counts/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(
		retest.pointer("/qmd_post_stage_counts/wrong_result").and_then(Value::as_u64),
		Some(13)
	);
	assert!(retest.pointer("/post_stage_basis").and_then(Value::as_str).is_some_and(|basis| {
		basis.contains("XY-955 closeout retest")
			&& basis.contains("qmd live adapter materialization is 17 pass")
	}));

	assert_dreaming_readiness_summary_buckets(ledger)
}

fn assert_dreaming_readiness_summary_buckets(ledger: &Value) -> Result<()> {
	assert!(support::array_contains_str(
		ledger,
		"/summary/improved",
		"current_vs_historical_correctness"
	)?);
	assert!(support::array_contains_str(ledger, "/summary/improved", "preference_evolution")?);
	assert!(support::array_contains_str(ledger, "/summary/improved", "reviewable_consolidation")?);
	assert!(support::array_contains_str(
		ledger,
		"/summary/improved",
		"memory_summary_top_of_mind_behavior"
	)?);
	assert!(support::array_contains_str(ledger, "/summary/improved", "proactive_brief_readiness")?);
	assert!(support::array_contains_str(
		ledger,
		"/summary/improved",
		"scheduled_memory_task_readiness"
	)?);
	assert!(support::array_at(ledger, "/summary/regressed")?.is_empty());
	assert!(support::array_contains_str(
		ledger,
		"/summary/unchanged",
		"deletion_ttl_tombstone_behavior"
	)?);
	assert!(support::array_contains_str(
		ledger,
		"/summary/unchanged",
		"final_competitor_retest_status"
	)?);
	assert!(support::array_at(ledger, "/summary/blocked")?.is_empty());
	assert!(support::array_at(ledger, "/summary/not_tested")?.is_empty());

	Ok(())
}

fn assert_dreaming_memory_summary_stage(stages: &[Value]) -> Result<()> {
	let summary_stage =
		support::find_by_field(stages, "/stage_id", "memory_summary_top_of_mind_behavior")?;

	assert_eq!(
		summary_stage.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(summary_stage.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(9));
	assert_eq!(
		summary_stage.pointer("/post_stage_counts/not_tested").and_then(Value::as_u64),
		Some(0)
	);
	assert!(
		summary_stage
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("fixture-backed memory_summary job")
				&& basis.contains("unsupported-claim flags"))
	);

	Ok(())
}

fn assert_dreaming_proactive_brief_stage(stages: &[Value]) -> Result<()> {
	let proactive_stage = support::find_by_field(stages, "/stage_id", "proactive_brief_readiness")?;

	assert_eq!(
		proactive_stage.pointer("/comparison_judgment").and_then(Value::as_str),
		Some("improved")
	);
	assert_eq!(proactive_stage.pointer("/post_stage_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(
		proactive_stage.pointer("/post_stage_counts/blocked").and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		proactive_stage.pointer("/post_stage_counts/evidence_ref_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		proactive_stage.pointer("/post_stage_counts/freshness_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		proactive_stage
			.pointer("/post_stage_counts/action_rationale_coverage")
			.and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		proactive_stage
			.pointer("/post_stage_counts/tombstone_violation_count")
			.and_then(Value::as_u64),
		Some(0)
	);
	assert!(
		proactive_stage
			.pointer("/post_stage_basis")
			.and_then(Value::as_str)
			.is_some_and(|basis| basis.contains("five proactive_brief fixture jobs")
				&& basis.contains("typed private-corpus refresh blocker"))
	);

	Ok(())
}
