pub(super) fn assert_trace_replay_diagnostics_markdown(markdown: &str) {
	assert!(markdown.contains("Retrieval correctness is still tied"));
	assert!(markdown.contains("| Default top-10 candidate artifact |"));
	assert!(markdown.contains("| Replay command locality |"));
	assert!(
		markdown
			.contains("| Operator-debug trace hydration | `live_real_world` | `pass` | `win` |")
	);
	assert!(markdown.contains(
		"| Operator-debug replay command availability | `live_real_world` | `pass` | `tie` |"
	));
	assert!(markdown.contains(
		"| Operator-debug candidate-drop visibility | `live_real_world` | `pass` | `win` |"
	));
	assert!(markdown.contains("| Rerank attribution | `live_baseline_only` | `non_goal` |"));
	assert!(markdown.contains("| Candidate-drop diagnostics | `research_gate` | `not_encoded` |"));
	assert!(markdown.contains("`retrieved_but_dropped` | Defined globally as `not_tested`"));
	assert!(markdown.contains("npx tsx src/cli/qmd.ts query"));
	assert!(markdown.contains("cargo run -p elf-eval -- --config-a"));
	assert!(markdown.contains("cargo make real-world-job-operator-ux-live-adapters"));
	assert!(markdown.contains("Do not claim qmd beats ELF as a memory system overall"));
	assert!(markdown.contains("Do not score rerank superiority from a qmd `--no-rerank` run"));
}
