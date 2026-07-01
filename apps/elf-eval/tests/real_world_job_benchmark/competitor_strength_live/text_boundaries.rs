pub(crate) fn assert_current_report_text_boundaries(
	measurement_audit: &str,
	competitor_matrix: &str,
	iteration_direction: &str,
	external_manifest: &str,
	comparison_external_projects: &str,
) {
	assert!(
		measurement_audit.contains(
			"| `memory_evolution` | `6` | `pass:1`, `wrong_result:5` | `wrong_result:6` |"
		)
	);
	assert!(
		measurement_audit
			.contains("qmd live fails 6/6 jobs after missing the delete/TTL tombstone evidence")
	);
	assert!(measurement_audit.contains("Basic local smoke and local OSS history/readback pass"));
	assert!(measurement_audit.contains("claude-mem hook/viewer capture is `blocked`"));
	assert!(!measurement_audit.contains("claude-mem hook/viewer capture remains untested"));
	assert!(!measurement_audit.contains("blocked or untested"));
	assert!(
		competitor_matrix
			.contains("broader live suites remain `wrong_result`, `blocked`, or `not_encoded`")
	);
	assert!(competitor_matrix.contains(
		"Overall adapter-status counts: 4 `pass`,\n6 `wrong_result`, 1 `lifecycle_fail`, 7 `blocked`, and 5 `not_encoded`."
	));
	assert!(!competitor_matrix.contains("5 `blocked`, and 7 `not_encoded`"));
	assert!(
		competitor_matrix
			.contains("mem0/OpenMemory local OSS entity-scoped personalization now passes")
	);
	assert!(competitor_matrix.contains("scoped preference behavior is a measured tie"));
	assert!(
		!competitor_matrix.contains("mem0/OpenMemory and Letta personalization are `not_encoded`")
	);
	assert!(external_manifest.contains(
		"The record is a full-suite sweep, not a full-suite pass; wrong_result, blocked, and not_encoded states remain visible."
	));
	assert!(external_manifest.contains(
		"The qmd live real-world sweep covers the current encoded fixture corpus; expanded retrieval-debug strength suites still need their own materialized adapter run."
	));
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for scoped local OSS same-corpus retrieval")
	);
	assert!(
		comparison_external_projects
			.contains("Benchmark-grounded for local same-corpus retrieval, reindex/update/delete")
	);
	assert!(iteration_direction.contains("| Jobs | `55` |"));
	assert!(iteration_direction.contains("| Encoded suites | `15` |"));
	assert!(iteration_direction.contains("| Pass | `49` |"));
	assert!(iteration_direction.contains("| Evidence coverage | `123/123` |"));
	assert!(iteration_direction.contains("| Expected evidence recall | `115/115` |"));

	for stale_phrase in [
		"same live sweep shape as ELF",
		"ELF and qmd live fail 5/6 jobs",
		"both systems currently fail 5/6 live memory-evolution jobs",
		"wrong_result, incomplete, blocked, and not_encoded states remain visible",
		"broader live suites remain `wrong_result`, `incomplete`, or `not_encoded`",
		"The qmd live real-world slice covers representative jobs only",
		"| Jobs | `40` |",
		"| Encoded suites | `11` |",
		"| Jobs | `50` |",
		"| Encoded suites | `14` |",
		"| Pass | `38` |",
		"| Pass | `45` |",
		"| Evidence coverage | `115/115` |",
		"| Expected evidence recall | `107/107` |",
		"history/UI/hosted/graph behavior remains",
		"current local adapter is incomplete/wrong-result",
		"current adapter is incomplete/invalid-result",
	] {
		assert!(!measurement_audit.contains(stale_phrase));
		assert!(!competitor_matrix.contains(stale_phrase));
		assert!(!iteration_direction.contains(stale_phrase));
		assert!(!external_manifest.contains(stale_phrase));
		assert!(!comparison_external_projects.contains(stale_phrase));
	}
}

pub(crate) fn assert_measurement_audit_adapter_status_counts(markdown: &str) {
	for expected in [
		"| `blocked` | `7` |",
		"| `not_encoded` | `5` |",
		"The generated JSON report emits `external_project_count: 16`",
	] {
		assert!(markdown.contains(expected), "missing measurement audit text: {expected}");
	}
	for stale in ["| `blocked` | `6` |", "| `not_encoded` | `6` |"] {
		assert!(!markdown.contains(stale), "stale measurement audit text: {stale}");
	}
}
