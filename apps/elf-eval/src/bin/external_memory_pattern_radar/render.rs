use color_eyre::{Result, eyre};

use crate::types::RadarCursor;

pub(super) fn render_summary(cursor: &RadarCursor) -> Result<String> {
	let run = cursor.last_run.as_ref().ok_or_else(|| eyre::eyre!("cursor has no last_run"))?;
	let last_verified = run.generated_at.get(..10).unwrap_or("unknown");
	let mut out = String::new();

	out.push_str("---\n");
	out.push_str("type: Evidence\n");
	out.push_str("title: \"External Memory Pattern Radar Summary\"\n");
	out.push_str("description: \"Latest weekly ELF external memory pattern radar outcome.\"\n");
	out.push_str("resource: docs/evidence/external_memory_pattern_radar_latest.md\n");
	out.push_str("status: active\n");
	out.push_str("authority: current_state\n");
	out.push_str("owner: evidence\n");
	out.push_str(&format!("last_verified: {last_verified}\n"));
	out.push_str("tags:\n");
	out.push_str("  - docs\n");
	out.push_str("  - external-memory-pattern-radar\n");
	out.push_str("  - evidence\n");
	out.push_str("source_refs: []\n");
	out.push_str("code_refs:\n");
	out.push_str("  - apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json\n");
	out.push_str("  - apps/elf-eval/src/bin/external_memory_pattern_radar.rs\n");
	out.push_str("related: []\n");
	out.push_str("drift_watch:\n");
	out.push_str("  - apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json\n");
	out.push_str("  - apps/elf-eval/src/bin/external_memory_pattern_radar.rs\n");
	out.push_str("---\n\n");
	out.push_str("# External Memory Pattern Radar Summary\n\n");
	out.push_str("Goal: Preserve the latest weekly ELF external memory pattern radar outcome.\n");
	out.push_str("Read this when: Feeding the next full comparison report or deciding whether a watched upstream memory project created an ELF follow-up.\n");
	out.push_str("Inputs: `apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json`, GitHub repository metadata, checked-in ELF comparison evidence, and any Codex source-review notes.\n");
	out.push_str("Depends on: `docs/spec/external_memory_pattern_radar_v1.md` and `docs/runbook/external_memory_pattern_radar.md`.\n");
	out.push_str("Outputs: Latest no-issue, rejection, or issue-ready radar decisions.\n\n");
	out.push_str(&format!("- Run id: `{}`\n", run.run_id));
	out.push_str(&format!("- Generated at: `{}`\n", run.generated_at));
	out.push_str(&format!("- Mode: `{}`\n", run.mode.as_str()));
	out.push_str(&format!(
		"- Projects: `{}`; covered: `{}`; rejected: `{}`; gaps: `{}`; create_issue: `{}`\n\n",
		run.summary.project_count,
		run.summary.covered_count,
		run.summary.rejected_count,
		run.summary.gap_count,
		run.summary.create_issue_count
	));
	out.push_str("## Decisions\n\n");
	out.push_str(
		"| Project | Upstream change | ELF verdict | Issue decision | Acceptance evidence |\n",
	);
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for decision in &run.decisions {
		out.push_str(&format!(
			"| `{}` | {} | `{}` | `{}` | {} |\n",
			decision.project_id,
			escape_markdown_table(&decision.upstream_change),
			decision.elf_verdict.as_str(),
			decision.issue_decision.action.as_str(),
			escape_markdown_table(&decision.acceptance_evidence.join("; "))
		));
	}

	out.push_str("\n## Safety Boundary\n\n");
	out.push_str("- The radar records upstream movement as a trigger for source review, not as proof of parity or a reason to adopt an external runtime.\n");
	out.push_str("- `create_issue` decisions are valid only when the cursor includes source links, repo evidence, non-goals, validation criteria, and Linear duplicate-search evidence.\n");
	out.push_str("- No-issue runs remain useful because each project records why ELF is already covered or why metadata-only movement was rejected.\n");

	Ok(out)
}

fn escape_markdown_table(value: &str) -> String {
	value.replace('|', "\\|").replace('\n', " ")
}
