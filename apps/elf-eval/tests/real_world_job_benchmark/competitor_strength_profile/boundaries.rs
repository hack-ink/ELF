use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_strength_profile_json_claim_boundaries(report: &Value) -> Result<()> {
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not broadly beat qmd; it ties encoded retrieval and lifecycle correctness, keeps qmd query transparency as not_tested for comparative scoring, and leaves replayability not_tested."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"qmd expansion, fusion, and rerank superiority remains not_tested because the current qmd paths use --no-rerank and do not score internals."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"ELF does not beat OpenViking on context trajectory; OpenViking trajectory strengths remain blocked/not_tested behind a wrong_result same-corpus output precondition and missing staged artifacts."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"Research_gate and blocked fixture records are follow-up gates, not pass evidence."
	)?);
	assert!(support::array_contains_str(
		report,
		"/claim_boundaries",
		"Missing equivalent surfaces are encoded as unsupported, blocked, or not_encoded rather than fake losses."
	)?);

	Ok(())
}

pub(crate) fn assert_strength_profile_markdown_boundaries(markdown: &str) {
	assert!(
		markdown.contains(
			"| Wrong-result diagnosis | `research_gate` | `not_encoded` | `not_tested` |"
		)
	);
	assert!(
		markdown.contains("ELF ties qmd on the current encoded retrieval-correctness surfaces")
	);
	assert!(markdown.contains("qmd remains the local retrieval-debug UX reference"));
	assert!(markdown.contains("not scored as comparative ELF wins or losses"));
	assert!(markdown.contains("ELF currently wins only the equivalent OpenViking same-corpus"));
	assert!(markdown.contains("Do not claim ELF broadly beats qmd"));
	assert!(markdown.contains(
		"Do not claim ELF beats OpenViking on staged retrieval, hierarchy, or recursive"
	));
	assert!(markdown.contains(
		"Do not turn `research_gate`, `blocked`, `not_encoded`, or `unsupported` surfaces"
	));
	assert!(markdown.contains("no pass evidence is claimed"));
	assert!(markdown.contains("typed `wrong_result` state"));
}

pub(crate) fn assert_operator_facing_strength_profile_boundaries(
	readme: &str,
	benchmarking_index: &str,
	iteration_direction: &str,
) {
	assert!(readme.contains("Full-suite live real-world adapter sweep after XY-926"));
	assert!(readme.contains("all 55 checked-in jobs across 13 suites"));
	assert!(readme.contains("ELF now live-scores capture/write-policy"));
	assert!(readme.contains("consolidation proposal review"));
	assert!(readme.contains("knowledge-page rebuild/lint"));
	assert!(readme.contains("operator-debugging fixtures"));
	assert!(!readme.contains("memory-evolution wrong results"));
	assert!(readme.contains("Live temporal reconciliation after XY-905"));
	assert!(readme.contains("now reports ELF live `memory_evolution` as 6/6 pass"));
	assert!(readme.contains("broad qmd, Graphiti/Zep, mem0/OpenMemory, Letta"));
	assert!(readme.contains("production-ops operator boundaries"));
	assert!(readme.contains("core/archival live adapter gap"));
	assert!(
		support::collapse_whitespace(readme).contains("blocked context-trajectory measurement")
	);
	assert!(
		readme
			.contains("consolidation, knowledge, capture, and core/archival typed non-pass states")
	);
	assert!(readme.contains("operator-debug trace hydration"));
	assert!(readme.contains("qmd remains the local retrieval-debug UX reference"));
	assert!(readme.contains("broad ELF-over-qmd"));
	assert!(readme.contains("qmd and OpenViking Strength-Profile Report - June 11, 2026"));
	assert!(benchmarking_index.contains("2026-06-11-qmd-openviking-strength-profile-report.md"));
	assert!(
		benchmarking_index.contains("separates qmd retrieval quality from debug/replay ergonomics")
	);
	assert!(benchmarking_index.contains("preserves XY-928 OpenViking"));
	assert!(
		benchmarking_index
			.contains("context-trajectory surfaces as blocked/not-tested until scored staged")
	);
	assert!(
		iteration_direction
			.contains("ELF and qmd are tied on the encoded live retrieval, work-resume, and")
	);
	assert!(iteration_direction.contains("ELF does not yet beat qmd's local retrieval-debug"));

	assert_iteration_direction_current_measurement_counts(iteration_direction);

	assert!(iteration_direction.contains(
		"ELF beats OpenViking on context trajectory. The scenario is encoded as blocked"
	));
	assert!(
		iteration_direction
			.contains("Do not promote a reference project into a win/loss claim until")
	);
}

fn assert_iteration_direction_current_measurement_counts(markdown: &str) {
	for expected in [
		"| Jobs | `55` |",
		"| Encoded suites | `15` |",
		"| Blocked | `6` |",
		"| Mean score | `0.891` |",
		"| Evidence coverage | `123/123` |",
		"| Source-ref coverage | `123/123` |",
		"| Quote coverage | `123/123` |",
		"| Expected evidence recall | `115/115` |",
		"| `blocked` | `7` |",
		"| `not_encoded` | `5` |",
		"`live_baseline_only`, `fixture_backed`, and `research_gate`",
		"`blocked` for fixture-backed trajectory gates",
	] {
		assert!(markdown.contains(expected), "missing iteration-direction text: {expected}");
	}
	for stale in [
		"| Jobs | `40` |",
		"| Encoded suites | `11` |",
		"| Jobs | `50` |",
		"| Encoded suites | `14` |",
		"| Mean score | `0.950` |",
		"| Mean score | `0.900` |",
		"| Evidence coverage | `88/88` |",
		"| Evidence coverage | `115/115` |",
		"| Expected evidence recall | `80/80` |",
		"| Expected evidence recall | `107/107` |",
		"| `blocked` | `5` |",
		"| `not_encoded` | `7` |",
		"`live_baseline_only` plus `research_gate`",
	] {
		assert!(!markdown.contains(stale), "stale iteration-direction text: {stale}");
	}
}
