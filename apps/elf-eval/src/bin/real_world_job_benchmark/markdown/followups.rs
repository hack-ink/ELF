use super::*;

pub(super) fn render_markdown_unsupported_claims(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Unsupported Claims\n\n");

	if report.unsupported_claims.is_empty() {
		out.push_str("No unsupported claims were produced by encoded jobs.\n\n");

		return;
	}

	out.push_str("| Suite | Job | Claim | Evidence | Reason |\n");
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for claim in &report.unsupported_claims {
		out.push_str(&format!(
			"| {} | {} | {} | `{}` | {} |\n",
			md_cell(claim.suite_id.as_str()),
			md_cell(claim.job_id.as_str()),
			md_cell(claim.claim_text.as_str()),
			md_inline(claim.evidence_ids.join(", ").as_str()),
			md_cell(claim.reason.as_str())
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_follow_ups(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Follow-Ups\n\n");

	if report.follow_ups.is_empty() {
		out.push_str("No benchmark follow-ups were declared by encoded jobs.\n\n");

		return;
	}

	out.push_str("| Suite | Job | Follow-up | Reason |\n");
	out.push_str("| --- | --- | --- | --- |\n");

	for follow_up in &report.follow_ups {
		out.push_str(&format!(
			"| {} | {} | {} | {} |\n",
			md_cell(follow_up.suite_id.as_str()),
			md_cell(follow_up.job_id.as_str()),
			md_cell(follow_up.title.as_str()),
			md_cell(follow_up.reason.as_str())
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_semantics(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Result Semantics\n\n");
	out.push_str(
		"This report uses `docs/spec/real_world_agent_memory_benchmark_v1.md` status terms.\n",
	);
	out.push_str("It is a real-world job fixture report, not a Docker live-baseline report.\n");
	out.push_str("Existing live-baseline reports remain valid for their encoded retrieval and lifecycle checks and are not reinterpreted as real-world suite wins.\n\n");
	out.push_str(
		"The summary counters report required evidence coverage, source-ref coverage, quote coverage, expected evidence recall, irrelevant context ratio, trace explainability, stale retrievals, scope violations, redaction leaks, Qdrant rebuild case coverage, stale answers, conflict detections, update rationale availability, and temporal validity gaps across encoded jobs.\n\n",
	);
	out.push_str(
		"- `pass`: encoded jobs met their pass threshold with required evidence and no hard-fail rule.\n",
	);
	out.push_str(
		"- `wrong_result`: a job completed but missed required answer or evidence expectations.\n",
	);
	out.push_str("- `incomplete`: the runner or adapter did not reach the behavioral check.\n");
	out.push_str("- `blocked`: required credentials, private input, product runtime, or host integration is outside the run scope.\n");
	out.push_str(
		"- `not_tested`: a comparison row or report slice has no executed benchmark evidence.\n",
	);
	out.push_str("- `unsupported_claim`: a job produced a substantive claim not supported by the fixture evidence links.\n");
	out.push_str("- `not_encoded`: a suite has no checked-in fixture, or an encoded fixture declares a capability gap so no pass/fail claim is allowed.\n");
	out.push_str(
		"- `fixture_backed`: checked-in fixtures were scored; no live product execution is implied.\n",
	);
	out.push_str("- `live_baseline`: Docker live-baseline retrieval or lifecycle evidence exists, but it is not a real-world suite pass by itself.\n");
	out.push_str("- `live_real_world`: a live adapter ran the real-world job contract and reported typed outcomes.\n");
	out.push_str("- `research_gate`: research, setup, source mapping, or resource gates are recorded before a fair benchmark can run.\n\n");
	out.push_str("Any `wrong_result`, `incomplete`, `blocked`, `not_tested`, `not_encoded`, `unsupported_claim`, or non-live evidence class must remain visible and must not be counted as a win.\n\n");
	out.push_str("For `knowledge_compilation` jobs, generated pages are benchmark artifacts. Page sections must cite source evidence or timeline events, or be explicitly flagged as unsupported. Flagged unsupported summaries are counted separately from hidden unsupported claims.\n\n");
	out.push_str("For `source_library` jobs, saved long-form material and social/thread captures are source records, not durable Memory Notes. Source records must preserve canonical source metadata, source_ref hydration pointers, and explicit promotion boundaries before any memory write is claimed.\n\n");
	out.push_str("For `memory_summary` jobs, summary artifacts are derived review surfaces. Top-of-mind entries must be current, included or downgraded entries must carry source refs, and derived project-profile entries must either cite sources or be explicitly flagged as unsupported.\n\n");
	out.push_str("For `proactive_brief` jobs, brief artifacts are fixture-scored derived outputs, not scheduled UI behavior. Every suggestion must carry evidence refs, freshness/currentness metadata, and an action rationale; stale, superseded, or tombstoned sources must not be presented as current recommendations.\n\n");
	out.push_str("For `scheduled_memory` jobs, task artifacts are deterministic fixture-scored stand-ins for asynchronous work. Every output must carry evidence refs, freshness/currentness metadata, action rationale, and execution trace/readback evidence; scheduled tasks must not mutate source notes silently or claim hosted scheduler/private-provider parity from fixture-only output.\n\n");
	out.push_str("For `work_continuity` jobs, Work Journal entries are source-adjacent readback artifacts, not current fact authority. Reset/resume, decisions, rejected options, next steps, handoff refs, redactions, and janitor candidates must preserve source refs and promotion boundaries; sensitive marker persistence, rejected-option resurrection, inferred next steps treated as instructions, and journal-only authority claims are hard fails.\n\n");
	out.push_str("## Suites With `not_encoded` Status\n\n");

	if report.not_encoded_suites.is_empty() {
		out.push_str("All declared suites have at least one encoded job.\n");
	} else {
		for suite in &report.not_encoded_suites {
			out.push_str(&format!("- `{}`\n", md_inline(suite.as_str())));
		}
	}
}
