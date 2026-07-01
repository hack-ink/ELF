use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::{closeout_reports::closeout_reports_helpers, support};

#[test]
fn p3_competitor_strength_absorption_report_preserves_claim_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::p3_competitor_strength_absorption_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::p3_competitor_strength_absorption_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.p3_competitor_strength_absorption_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1072"));
	assert_eq!(
		report.pointer("/self_assessment/verdict").and_then(Value::as_str),
		Some("pass_with_p4_queue_ready_after_main_thread_acceptance")
	);
	assert_eq!(
		report.pointer("/self_assessment/p4_queued_label_applied").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report
			.pointer("/self_assessment/typed_non_pass_states_are_not_wins")
			.and_then(Value::as_bool),
		Some(true)
	);

	let products = support::array_at(&report, "/product_strengths")?;

	for product in [
		"qmd",
		"VectifyAI PageIndex",
		"VectifyAI OpenKB",
		"mem0/OpenMemory",
		"Letta",
		"Graphiti/Zep",
		"OpenViking",
		"RAGFlow",
		"GraphRAG",
		"LightRAG",
	] {
		support::find_by_field(products, "/product", product)?;
	}

	let qmd = support::find_by_field(products, "/product", "qmd")?;
	let pageindex = support::find_by_field(products, "/product", "VectifyAI PageIndex")?;
	let mem0 = support::find_by_field(products, "/product", "mem0/OpenMemory")?;
	let graphiti = support::find_by_field(products, "/product", "Graphiti/Zep")?;
	let lightrag = support::find_by_field(products, "/product", "LightRAG")?;

	assert_eq!(qmd.pointer("/current_status").and_then(Value::as_str), Some("mixed"));
	assert!(
		qmd.pointer("/remains_stronger_elsewhere")
			.and_then(Value::as_str)
			.is_some_and(|value| value.contains("top-k JSON"))
	);
	assert_eq!(pageindex.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		mem0.pointer("/current_status").and_then(Value::as_str),
		Some("split_pass_and_blocked")
	);
	assert_eq!(graphiti.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		lightrag.pointer("/current_status").and_then(Value::as_str),
		Some("incomplete_or_not_encoded")
	);

	let queue = support::array_at(&report, "/p4_optimization_queue")?;

	for key in [
		"qmd_candidate_replay_parity",
		"adapter_outcome_grammar_and_metrics",
		"source_library_tree_and_wiki_adapters",
		"memory_history_export_and_core_archive",
		"temporal_trajectory_graph_rag_adapters",
	] {
		let item = support::find_by_field(queue, "/key", key)?;

		assert_eq!(
			item.pointer("/ready_after_main_thread_acceptance").and_then(Value::as_bool),
			Some(true)
		);
		assert_eq!(item.pointer("/queued_label_applied").and_then(Value::as_bool), Some(false));
	}

	closeout_reports_helpers::assert_product_queue_items_reference_queue(products, queue)?;

	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Typed non-pass states are not wins."
	)?);
	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not apply decodex:queued:elf to a P4 issue until the main thread accepts the P3 closeout."
	)?);
	assert!(markdown.contains("P3 is decision-ready for main-thread inspection"));
	assert!(markdown.contains("Typed non-pass states are not wins"));
	assert!(markdown.contains("No P4 issue receives `decodex:queued:elf`"));
	assert!(benchmarking_index.contains("2026-06-23-p3-competitor-strength-absorption-report.md"));
	assert!(readme.contains("P3 competitor-strength absorption closeout after XY-1072"));
	assert!(readme.contains("`decodex:queued:elf` label"));

	Ok(())
}
