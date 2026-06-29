use super::*;

pub(super) fn render_markdown_suites(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Suites\n\n");
	out.push_str(
		"| Suite | Status | Jobs | Score | Evidence Recall | Irrelevant Context | Trace Explain | Stale Answers | Conflicts | Update Rationales | Temporal Gaps | History Readback | Unsupported Claims | Wrong Results | Reason |\n",
	);
	out.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n");

	for suite in &report.suites {
		out.push_str(&format!(
			"| {} | `{}` | {} | `{}` | `{}` | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
			md_cell(suite.suite_id.as_str()),
			status_str(suite.status),
			suite.encoded_job_count,
			optional_f64(suite.score_mean, ""),
			optional_f64(suite.expected_evidence_recall, ""),
			optional_f64(suite.irrelevant_context_ratio, ""),
			suite.trace_explainability_count,
			suite.stale_answer_count,
			suite.conflict_detection_count,
			suite.update_rationale_available_count,
			suite.temporal_validity_not_encoded_count,
			suite.history_readback_encoded_count,
			suite.unsupported_claim_count,
			suite.wrong_result_count,
			md_cell(suite.reason.as_str())
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_jobs(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Jobs\n\n");
	out.push_str("| Suite | Job | Status | Answer Type | Caveat Required | Refusal Required | Unknown Allowed | Score | Evidence Recall | Irrelevant Context | Expected Evidence | Produced Evidence | Trace Failure Stage | Stale Answers | Conflicts | Update Rationale | Temporal Gap | Unsupported Claims | Wrong Results | Latency | Cost |\n");
	out.push_str(
		"| --- | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | --- | --- | ---: | ---: | ---: | --- |\n",
	);

	for job in &report.jobs {
		let expected = job
			.expected_evidence
			.iter()
			.map(|evidence| evidence.evidence_id.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		let produced = job.produced_evidence.join(", ");

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | `{:.3}` | `{}` | `{}` | `{}` | {} | {} | `{}` | `{}` | {} | {} | `{}` | `{}` |\n",
			md_cell(job.suite_id.as_str()),
			md_cell(job.job_id.as_str()),
			status_str(job.status),
			md_inline(job.answer_type.as_str()),
			bool_display(job.requires_caveat),
			bool_display(job.requires_refusal),
			bool_display(job.can_answer_unknown),
			job.normalized_score,
			job.retrieval_quality.expected_evidence_recall,
			job.retrieval_quality.irrelevant_context_ratio,
			md_inline(expected.as_str()),
			md_inline(produced.as_str()),
			md_inline(trace_failure_stage(job.trace_explainability.as_ref()).unwrap_or("-")),
			job.stale_answer_count,
			job.conflict_detection_count,
			bool_display(job.update_rationale_available),
			bool_display(job.temporal_validity_not_encoded),
			job.unsupported_claim_count,
			job.wrong_result_count,
			optional_f64(job.latency_ms, " ms"),
			cost_display(job.cost.as_ref())
		));
	}

	out.push('\n');
}

pub(super) fn render_markdown_operator_debugging(out: &mut String, report: &RealWorldReport) {
	let jobs = report.jobs.iter().filter(|job| job.operator_debug.is_some()).collect::<Vec<_>>();

	out.push_str("## Operator Debugging UX\n\n");

	if jobs.is_empty() {
		out.push_str("No encoded job reported operator debugging evidence.\n\n");

		return;
	}

	out.push_str("| Job | Failure Mode | Trace Evidence | Trace Available | Replay Command | Steps | Raw SQL | Dropped Candidate Visibility | Trace Completeness | Repair Clarity | UX Gaps |\n");
	out.push_str("| --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- | --- |\n");

	for job in jobs {
		if let Some(debug) = &job.operator_debug {
			out.push_str(&format!(
				"| {} | {} | {} | `{}` | `{}` | {} | `{}` | {} | `{}` | `{}` | {} |\n",
				md_cell(job.job_id.as_str()),
				md_cell(debug.failure_mode.as_str()),
				debug_trace_cell(debug),
				debug.trace_available.unwrap_or(debug.trace_id.is_some()),
				debug.replay_command_available.unwrap_or(debug.replay_command.is_some()),
				debug.steps_to_root_cause,
				debug.raw_sql_needed,
				md_cell(debug.dropped_candidate_visibility.as_str()),
				md_inline(debug.trace_completeness.as_str()),
				md_inline(debug.repair_action_clarity.as_str()),
				ux_gap_cell(debug.ux_gaps.as_slice())
			));
		}
	}

	out.push_str("\n### Operator Debug Details\n\n");

	for job in report.jobs.iter().filter(|job| job.operator_debug.is_some()) {
		if let Some(debug) = &job.operator_debug {
			out.push_str(&format!("#### `{}`\n\n", md_inline(job.job_id.as_str())));
			out.push_str(&format!("- Root cause: {}\n", md_cell(debug.root_cause.as_str())));
			out.push_str(&format!(
				"- Viewer panels: `{}`\n",
				md_inline(debug.viewer_panels.join(", ").as_str())
			));
			out.push_str(&format!(
				"- CLI steps: `{}`\n",
				md_inline(debug.cli_steps.join(" -> ").as_str())
			));

			if let Some(command) = &debug.replay_command {
				out.push_str(&format!("- Replay command: `{}`\n", md_inline(command.as_str())));
			}
			if let Some(artifact) = &debug.replay_artifact {
				out.push_str(&format!("- Replay artifact: `{}`\n", md_inline(artifact.as_str())));
			}

			out.push_str(&format!(
				"- Trace evidence: `{}`\n",
				md_inline(debug.trace_evidence.join(", ").as_str())
			));
			out.push('\n');
		}
	}
}

fn debug_trace_cell(debug: &OperatorDebugEvidence) -> String {
	let trace = debug.trace_id.as_deref().unwrap_or("-");
	let viewer = debug
		.viewer_url
		.as_deref()
		.map(|url| format!("[viewer]({})", md_url(url)))
		.unwrap_or_else(|| "viewer: -".to_string());
	let bundle = debug
		.admin_trace_bundle_url
		.as_deref()
		.map(|url| format!("[bundle]({})", md_url(url)))
		.unwrap_or_else(|| "bundle: -".to_string());

	format!("`{}`<br>{}<br>{}", md_inline(trace), viewer, bundle)
}

fn ux_gap_cell(gaps: &[OperatorUxGap]) -> String {
	if gaps.is_empty() {
		return "`none`".to_string();
	}

	gaps.iter()
		.map(|gap| {
			format!(
				"`{}`: {} ({})",
				md_inline(gap.gap_id.as_str()),
				md_cell(gap.description.as_str()),
				md_inline(gap.follow_up_issue.as_str())
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}
