use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn dreaming_competitor_strength_retest_report_closes_xy955_without_overclaims() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::dreaming_competitor_strength_retest_report_json_path()?,
	)?)?;
	let markdown =
		fs::read_to_string(support::dreaming_competitor_strength_retest_report_markdown_path()?)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.dreaming_competitor_strength_retest_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-955"));
	assert_eq!(
		report.pointer("/summary/overall_judgment").and_then(Value::as_str),
		Some("locally_and_partially_stronger_only")
	);
	assert_eq!(
		report.pointer("/summary/broader_superiority").and_then(Value::as_str),
		Some("not_proven")
	);
	assert_eq!(report.pointer("/summary/regressed_stage_count").and_then(Value::as_u64), Some(0));
	assert!(support::array_contains_str(&report, "/status_terms", "typed_non_pass")?);
	assert!(support::array_contains_str(
		&report,
		"/summary/unsupported_claims_rejected",
		"ELF does not broadly beat qmd from this retest."
	)?);

	assert_xy955_commands(&report)?;
	assert_xy955_stage_closeout(&report)?;
	assert_xy955_scenario_retests(&report)?;
	assert_xy955_optimization_queue(&report)?;
	assert_xy955_follow_up_issue_briefs(&report)?;

	assert!(markdown.contains("ELF is locally and partially stronger"));
	assert!(
		markdown.contains("The full live-adapter command now has fresh ELF and qmd scored reports")
	);
	assert!(
		markdown.contains(
			"Do not treat qmd full-suite wrong_result counts as a regression of qmd debug"
		)
	);
	assert!(markdown.contains("## Follow-Up Issue Briefs"));
	assert!(markdown.contains(
		"| GraphRAG/LightRAG/RAGFlow/llm-wiki/gbrain/graphify citation/navigation/knowledge surfaces |"
	));
	assert!(
		benchmarking_index.contains("2026-06-17-dreaming-competitor-strength-retest-report.md")
	);
	assert!(readme.contains("Dreaming Competitor-Strength Retest Report - June 17, 2026"));
	assert!(readme.contains("17 competitor-strength closeout"));

	Ok(())
}

fn assert_xy955_commands(report: &Value) -> Result<()> {
	let commands = support::array_at(report, "/commands")?;
	let aggregate = support::find_by_field(commands, "/command", "cargo make real-world-memory")?;
	let graph_rag =
		support::find_by_field(commands, "/command", "cargo make real-world-memory-graph-rag")?;
	let first_generation =
		support::find_by_field(commands, "/command", "cargo make real-world-first-generation-oss")?;
	let live =
		support::find_by_field(commands, "/command", "cargo make real-world-memory-live-adapters")?;

	assert_eq!(aggregate.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(aggregate.pointer("/summary/pass").and_then(Value::as_u64), Some(53));
	assert_eq!(aggregate.pointer("/summary/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(graph_rag.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(graph_rag.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(1));
	assert_eq!(graph_rag.pointer("/summary/incomplete").and_then(Value::as_u64), Some(1));
	assert_eq!(graph_rag.pointer("/summary/blocked").and_then(Value::as_u64), Some(3));
	assert_eq!(first_generation.pointer("/summary/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(first_generation.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(live.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		live.pointer("/partial_summary/elf_live_real_world/pass").and_then(Value::as_u64),
		Some(40)
	);
	assert_eq!(
		live.pointer("/partial_summary/elf_live_real_world/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		live.pointer("/partial_summary/qmd_live_real_world/pass").and_then(Value::as_u64),
		Some(17)
	);
	assert_eq!(
		live.pointer("/partial_summary/qmd_live_real_world/wrong_result").and_then(Value::as_u64),
		Some(13)
	);

	Ok(())
}

fn assert_xy955_stage_closeout(report: &Value) -> Result<()> {
	let stages = support::array_at(report, "/stage_closeout")?;

	assert_eq!(stages.len(), 8);

	let current = support::find_by_field(stages, "/stage_id", "current_vs_historical_correctness")?;
	let proactive = support::find_by_field(stages, "/stage_id", "proactive_brief_readiness")?;
	let scheduled = support::find_by_field(stages, "/stage_id", "scheduled_memory_task_readiness")?;
	let final_retest =
		support::find_by_field(stages, "/stage_id", "final_competitor_retest_status")?;

	assert_eq!(current.pointer("/judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(current.pointer("/current_counts/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(current.pointer("/current_counts/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(proactive.pointer("/judgment").and_then(Value::as_str), Some("improved"));
	assert_eq!(proactive.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(scheduled.pointer("/current_counts/pass").and_then(Value::as_u64), Some(4));
	assert_eq!(scheduled.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(final_retest.pointer("/judgment").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(final_retest.pointer("/current_counts/pass").and_then(Value::as_u64), Some(40));
	assert_eq!(
		final_retest.pointer("/current_counts/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(final_retest.pointer("/current_counts/blocked").and_then(Value::as_u64), Some(7));
	assert_eq!(
		final_retest.pointer("/current_counts/not_encoded").and_then(Value::as_u64),
		Some(19)
	);
	assert!(final_retest.pointer("/boundary").and_then(Value::as_str).is_some_and(|boundary| {
		boundary.contains("qmd now has a fresh scored live report")
			&& boundary.contains("broader superiority is not proven")
	}));
	assert_eq!(final_retest.pointer("/qmd_current_counts/pass").and_then(Value::as_u64), Some(17));
	assert_eq!(
		final_retest.pointer("/qmd_current_counts/wrong_result").and_then(Value::as_u64),
		Some(13)
	);

	Ok(())
}

fn assert_xy955_scenario_retests(report: &Value) -> Result<()> {
	let scenarios = support::array_at(report, "/scenario_retests")?;
	let qmd = support::find_by_field(scenarios, "/scenario_id", "qmd_debug_ergonomics")?;
	let mem0 = support::find_by_field(
		scenarios,
		"/scenario_id",
		"mem0_openmemory_preference_history_export",
	)?;
	let letta = support::find_by_field(scenarios, "/scenario_id", "letta_core_archive")?;
	let graph_rag = support::find_by_field(
		scenarios,
		"/scenario_id",
		"graph_rag_citation_navigation_knowledge_surfaces",
	)?;
	let private_provider =
		support::find_by_field(scenarios, "/scenario_id", "private_provider_production_gates")?;

	assert_eq!(qmd.pointer("/current_outcome").and_then(Value::as_str), Some("unchanged"));
	assert_eq!(qmd.pointer("/current_status").and_then(Value::as_str), Some("pass"));
	assert!(qmd.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("17 pass")
			&& evidence.contains("13 wrong_result")
			&& evidence.contains("does not retest or erase")
	}));
	assert_eq!(mem0.pointer("/current_outcome").and_then(Value::as_str), Some("unchanged"));
	assert!(mem0.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("mem0/OpenMemory local OSS history")
			&& evidence.contains("OpenMemory UI/export remains setup-blocked")
	}));
	assert_eq!(letta.pointer("/current_status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		graph_rag.pointer("/current_status").and_then(Value::as_str),
		Some("typed_non_pass")
	);
	assert!(graph_rag.pointer("/evidence").and_then(Value::as_str).is_some_and(|evidence| {
		evidence.contains("0 pass")
			&& evidence.contains("1 wrong_result")
			&& evidence.contains("3 blocked")
	}));
	assert_eq!(private_provider.pointer("/follow_up").and_then(Value::as_str), Some("XY-930"));

	Ok(())
}

fn assert_xy955_optimization_queue(report: &Value) -> Result<()> {
	let queue = support::array_at(report, "/optimization_queue")?;
	let qmd = support::find_by_field(queue, "/issue", "XY-923")?;
	let private_provider = support::find_by_field(queue, "/issue", "XY-930")?;
	let openviking = support::find_by_field(queue, "/issue", "XY-928")?;
	let letta = support::find_by_field(queue, "/issue", "letta-core-archive-adapter-brief")?;
	let service_native =
		support::find_by_field(queue, "/issue", "service-native-dreaming-outputs-brief")?;

	assert_eq!(qmd.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(private_provider.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(openviking.pointer("/status").and_then(Value::as_str), Some("existing"));
	assert_eq!(letta.pointer("/status").and_then(Value::as_str), Some("proposed"));
	assert_eq!(service_native.pointer("/status").and_then(Value::as_str), Some("proposed"));
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries/not_allowed",
		"Do not treat qmd full-suite wrong_result counts as a regression of qmd debug ergonomics."
	)?);

	Ok(())
}

fn assert_xy955_follow_up_issue_briefs(report: &Value) -> Result<()> {
	let existing = support::array_at(report, "/follow_up_issue_briefs/existing")?;
	let proposed = support::array_at(report, "/follow_up_issue_briefs/proposed")?;
	let qmd = support::find_by_field(existing, "/issue", "XY-923")?;
	let private_provider = support::find_by_field(existing, "/issue", "XY-930")?;
	let letta = support::find_by_field(proposed, "/issue", "letta-core-archive-adapter-brief")?;
	let service_native =
		support::find_by_field(proposed, "/issue", "service-native-dreaming-outputs-brief")?;

	assert!(qmd.pointer("/scope").and_then(Value::as_str).is_some_and(|scope| {
		scope.contains("immediate top-k") && scope.contains("candidate-drop artifacts")
	}));
	assert!(qmd.pointer("/non_goal").and_then(Value::as_str).is_some_and(|non_goal| {
		non_goal.contains("qmd full-suite wrong_result counts")
			&& non_goal.contains("debug ergonomics")
	}));
	assert!(
		private_provider
			.pointer("/non_goal")
			.and_then(Value::as_str)
			.is_some_and(|non_goal| non_goal.contains("Do not infer credentials"))
	);
	assert!(letta.pointer("/validation").and_then(Value::as_str).is_some_and(|validation| {
		validation.contains("Letta core block JSON") && validation.contains("typed outcome states")
	}));
	assert!(
		service_native
			.pointer("/non_goal")
			.and_then(Value::as_str)
			.is_some_and(|non_goal| non_goal.contains("Pulse clone"))
	);

	Ok(())
}
