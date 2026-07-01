use std::fs;

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn p2_knowledge_workspace_closeout_preserves_pageindex_openkb_boundaries() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::p2_knowledge_workspace_pageindex_openkb_closeout_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(
		support::p2_knowledge_workspace_pageindex_openkb_closeout_report_markdown_path()?,
	)?;
	let makefile = fs::read_to_string(support::workspace_root()?.join("Makefile.toml"))?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;
	let benchmark_runbook = fs::read_to_string(
		support::workspace_root()?
			.join("docs")
			.join("runbook")
			.join("benchmarking")
			.join("real_world_agent_memory_benchmark.md"),
	)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.p2_knowledge_workspace_pageindex_openkb_closeout_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-1066"));
	assert_eq!(
		report.pointer("/self_assessment/verdict").and_then(Value::as_str),
		Some("pass_with_reference_only_competitor_boundary")
	);
	assert_eq!(report.pointer("/typed_state_summary/pass").and_then(Value::as_u64), Some(2));
	assert_eq!(
		report.pointer("/typed_state_summary/wrong_result").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/typed_state_summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/typed_state_summary/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(report.pointer("/typed_state_summary/not_tested").and_then(Value::as_u64), Some(2));

	let results = support::array_at(&report, "/elf_same_corpus_results")?;
	let source_library = support::find_by_field(results, "/suite", "source_library")?;
	let knowledge = support::find_by_field(results, "/suite", "knowledge_compilation")?;

	assert_eq!(source_library.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(source_library.pointer("/jobs").and_then(Value::as_u64), Some(2));
	assert_eq!(knowledge.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(knowledge.pointer("/jobs").and_then(Value::as_u64), Some(3));
	assert!(support::array_contains_str(
		knowledge,
		"/coverage",
		"Changed-source watch/rebuild reports changed, stale, and reviewable memory-candidate outputs without source mutation."
	)?);

	let matrix = support::array_at(&report, "/comparison_matrix")?;
	let pageindex = support::find_by_field(matrix, "/target", "VectifyAI PageIndex")?;
	let openkb = support::find_by_field(matrix, "/target", "VectifyAI OpenKB")?;
	let p3 = support::find_by_field(matrix, "/target", "P3 PageIndex/OpenKB adapter queue")?;

	assert_eq!(pageindex.pointer("/status").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(openkb.pointer("/status").and_then(Value::as_str), Some("not_tested"));
	assert_eq!(p3.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		report
			.pointer("/p3_queue_decision/ready_to_queue_after_main_thread_acceptance")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report.pointer("/p3_queue_decision/queued_label_applied").and_then(Value::as_bool),
		Some(false)
	);
	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim ELF beats PageIndex or OpenKB."
	)?);
	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not queue a P3 issue in this lane."
	)?);
	assert!(markdown.contains("P2 Knowledge Workspace PageIndex/OpenKB Closeout Report"));
	assert!(markdown.contains("VectifyAI PageIndex"));
	assert!(markdown.contains("VectifyAI OpenKB"));
	assert!(markdown.contains("This report does not apply `decodex:queued:elf`"));
	assert!(makefile.contains("[tasks.real-world-memory-p2-knowledge-closeout]"));
	assert!(makefile.contains("\"real-world-memory-source-library-report\""));
	assert!(makefile.contains("\"real-world-memory-knowledge-report\""));
	assert!(
		benchmarking_index
			.contains("2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md")
	);
	assert!(readme.contains("P2 Knowledge Workspace PageIndex/OpenKB closeout after XY-1066"));
	assert!(readme.contains("real-world-memory-p2-knowledge-closeout"));
	assert!(benchmark_runbook.contains("cargo make real-world-memory-p2-knowledge-closeout"));

	Ok(())
}

#[test]
fn operator_approved_public_proxy_private_addendum_preserves_boundary() -> Result<()> {
	let report = serde_json::from_str::<Value>(&fs::read_to_string(
		support::operator_approved_public_proxy_private_addendum_report_json_path()?,
	)?)?;
	let markdown = fs::read_to_string(
		support::operator_approved_public_proxy_private_addendum_report_markdown_path()?,
	)?;
	let benchmarking_index = fs::read_to_string(support::benchmarking_index_path()?)?;
	let readme = fs::read_to_string(support::readme_path()?)?;

	assert_eq!(
		report.pointer("/schema").and_then(Value::as_str),
		Some("elf.operator_approved_public_proxy_baseline_report/v1")
	);
	assert_eq!(report.pointer("/authority").and_then(Value::as_str), Some("XY-930"));
	assert_eq!(report.pointer("/command/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		report.pointer("/command/run_id").and_then(Value::as_str),
		Some("live-baseline-20260619143959")
	);
	assert_eq!(
		report.pointer("/corpus/profile").and_then(Value::as_str),
		Some("production-private")
	);
	assert_eq!(
		report.pointer("/corpus/runner_track").and_then(Value::as_str),
		Some("private_production")
	);
	assert_eq!(
		report.pointer("/corpus/manifest_kind").and_then(Value::as_str),
		Some("operator_approved_public_proxy")
	);
	assert_eq!(
		report.pointer("/corpus/manifest_id").and_then(Value::as_str),
		Some("operator-approved-public-proxy-prod-corpus-2026-06-19")
	);
	assert_eq!(report.pointer("/embedding/mode").and_then(Value::as_str), Some("local"));
	assert_eq!(
		report.pointer("/embedding/provider_backed_quality_proven").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(report.pointer("/summary/project_status").and_then(Value::as_str), Some("pass"));
	assert_eq!(report.pointer("/summary/wrong_result").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/check_summary/total").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/check_summary/pass").and_then(Value::as_u64), Some(8));
	assert_eq!(
		report.pointer("/query_summary/wrong_result_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(report.pointer("/backfill/completed_count").and_then(Value::as_u64), Some(12));
	assert_eq!(report.pointer("/backfill/duplicate_source_notes").and_then(Value::as_u64), Some(0));

	let queries = support::array_at(&report, "/queries")?;
	let provider = support::find_by_field(queries, "/id", "q-explain-provider-blocker")?;

	assert_eq!(queries.len(), 8);
	assert_eq!(
		provider.pointer("/top_evidence").and_then(Value::as_str),
		Some("blocker-provider-missing")
	);
	assert_eq!(provider.pointer("/matched").and_then(Value::as_bool), Some(true));
	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not call this real private-corpus production proof."
	)?);
	assert!(support::array_contains_str(
		&report,
		"/claim_boundaries/not_allowed",
		"Do not claim provider-backed production quality; embedding mode was local."
	)?);
	assert!(support::array_contains_str(
		&report,
		"/improvement_regression_readback/unchanged",
		"Real private-corpus production quality is still not proven."
	)?);
	assert!(support::array_contains_str(
		&report,
		"/next_optimization_direction/when_operator_inputs_exist",
		"Run provider-backed embeddings with ELF_BASELINE_ELF_EMBEDDING_MODE=provider and a routed provider setup."
	)?);
	assert!(markdown.contains("proxy corpus pass"));
	assert!(markdown.contains("Do not call this real private-corpus production proof."));
	assert!(markdown.contains("| Embedding mode | `local` |"));
	assert!(
		benchmarking_index
			.contains("2026-06-19-operator-approved-public-proxy-production-private-addendum.md")
	);
	assert!(benchmarking_index.contains("not real private-corpus or provider-backed proof"));
	assert!(readme.contains("Operator-approved public-proxy addendum after XY-930"));
	assert!(readme.contains("8/8 query passes"));
	assert!(readme.contains("does not prove real private-corpus production quality"));

	Ok(())
}
