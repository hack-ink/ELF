pub(super) fn assert_dreaming_readiness_markdown_boundaries(markdown: &str) {
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
