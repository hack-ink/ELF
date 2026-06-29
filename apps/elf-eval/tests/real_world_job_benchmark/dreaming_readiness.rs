use std::fs;

use color_eyre::{Result, eyre};
use serde_json::Value;

use super::support::*;

#[test]
fn dreaming_readiness_stage_ledger_preserves_gate_shape() -> Result<()> {
	let ledger = serde_json::from_str::<Value>(&fs::read_to_string(
		dreaming_readiness_stage_ledger_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(dreaming_readiness_stage_ledger_markdown_path()?)?;
	let stages = array_at(&ledger, "/stage_gates")?;

	assert_dreaming_readiness_ledger_header(&ledger)?;
	assert_dreaming_readiness_stage_shape(&ledger, stages)?;
	assert_dreaming_readiness_baseline_counts(&ledger, stages)?;
	assert_dreaming_readiness_markdown_boundaries(&markdown);

	Ok(())
}

fn assert_dreaming_readiness_ledger_header(ledger: &Value) -> Result<()> {
	assert_eq!(
		ledger.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_readiness_stage_ledger/v1")
	);
	assert_eq!(ledger.pointer("/authority").and_then(Value::as_str), Some("XY-951"));

	for term in ["improved", "regressed", "unchanged", "blocked", "not_tested"] {
		assert!(array_contains_str(ledger, "/judgment_terms", term)?);
	}
	for term in ["pass", "wrong_result", "blocked", "not_tested", "not_encoded"] {
		assert!(array_contains_str(ledger, "/count_fields", term)?);
	}

	Ok(())
}

fn assert_dreaming_readiness_stage_shape(ledger: &Value, stages: &[Value]) -> Result<()> {
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
		find_by_field(stages, "/stage_id", stage_id)?;
	}
	for stage in stages {
		let stage_id =
			stage.pointer("/stage_id").and_then(Value::as_str).unwrap_or("<missing stage_id>");

		assert!(
			!array_at(stage, "/baseline_commands")?.is_empty(),
			"{stage_id} missing baseline commands"
		);
		assert!(
			!array_at(stage, "/post_stage_commands")?.is_empty(),
			"{stage_id} missing post-stage commands"
		);
		assert!(
			!array_at(stage, "/evidence_files")?.is_empty(),
			"{stage_id} missing evidence files"
		);

		for count_field in string_array_at(ledger, "/count_fields")? {
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

		assert!(array_contains_str(ledger, "/judgment_terms", judgment)?);
	}

	Ok(())
}

fn assert_dreaming_readiness_baseline_counts(ledger: &Value, stages: &[Value]) -> Result<()> {
	let current = find_by_field(stages, "/stage_id", "current_vs_historical_correctness")?;

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

	let preference = find_by_field(stages, "/stage_id", "preference_evolution")?;

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

	let tombstone = find_by_field(stages, "/stage_id", "deletion_ttl_tombstone_behavior")?;

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

	let consolidation = find_by_field(stages, "/stage_id", "reviewable_consolidation")?;

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

	let scheduled = find_by_field(stages, "/stage_id", "scheduled_memory_task_readiness")?;

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
	let retest = find_by_field(stages, "/stage_id", "final_competitor_retest_status")?;

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
	assert!(array_contains_str(ledger, "/summary/improved", "current_vs_historical_correctness")?);
	assert!(array_contains_str(ledger, "/summary/improved", "preference_evolution")?);
	assert!(array_contains_str(ledger, "/summary/improved", "reviewable_consolidation")?);
	assert!(array_contains_str(
		ledger,
		"/summary/improved",
		"memory_summary_top_of_mind_behavior"
	)?);
	assert!(array_contains_str(ledger, "/summary/improved", "proactive_brief_readiness")?);
	assert!(array_contains_str(ledger, "/summary/improved", "scheduled_memory_task_readiness")?);
	assert!(array_at(ledger, "/summary/regressed")?.is_empty());
	assert!(array_contains_str(ledger, "/summary/unchanged", "deletion_ttl_tombstone_behavior")?);
	assert!(array_contains_str(ledger, "/summary/unchanged", "final_competitor_retest_status")?);
	assert!(array_at(ledger, "/summary/blocked")?.is_empty());
	assert!(array_at(ledger, "/summary/not_tested")?.is_empty());

	Ok(())
}

fn assert_dreaming_memory_summary_stage(stages: &[Value]) -> Result<()> {
	let summary_stage = find_by_field(stages, "/stage_id", "memory_summary_top_of_mind_behavior")?;

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
	let proactive_stage = find_by_field(stages, "/stage_id", "proactive_brief_readiness")?;

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

fn assert_dreaming_readiness_markdown_boundaries(markdown: &str) {
	assert!(
		markdown.contains("`improved`: current-vs-historical correctness, preference evolution")
			&& markdown.contains("reviewable")
			&& markdown.contains("proactive brief")
	);
	assert!(markdown.contains("memory-summary/top-of-mind fixture readback"));
	assert!(markdown.contains("XY-953 adds a direct `proactive_brief` suite"));
	assert!(markdown.contains("XY-954 adds a direct `scheduled_memory` suite"));
	assert!(markdown.contains(
		"Do not claim fixture-backed proactive brief scoring proves OpenAI Pulse parity"
	));
	assert!(
		markdown
			.contains("Do not claim fixture-backed scheduled-memory scoring proves ChatGPT Tasks")
	);
	assert!(markdown.contains("`regressed`: none"));
	assert!(markdown.contains("the XY-905 run passes all six memory-evolution jobs"));
	assert!(markdown.contains("XY-952 adds a reviewable `elf.memory_summary/v1`"));
	assert!(markdown.contains("XY-955 closes the final competitor retest row"));
	assert!(markdown.contains("XY-905"));
	assert!(markdown.contains("qmd live `pass=17`, `wrong_result=13`"));
	assert!(
		markdown
			.contains("Do not claim this ledger proves preference history against mem0/OpenMemory")
	);
	assert!(markdown.contains("Reviewable consolidation now has ELF live service-backed"));
}
