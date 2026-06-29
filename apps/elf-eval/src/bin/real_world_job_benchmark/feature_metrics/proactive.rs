use super::*;

pub(super) fn proactive_brief_metrics_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<ProactiveBriefJobMetrics> {
	if answer.proactive_briefs.is_empty() {
		return None;
	}

	let mut metrics = ProactiveBriefJobMetrics {
		brief_count: answer.proactive_briefs.len(),
		required_suggestion_kind_count: job
			.proactive_brief
			.as_ref()
			.map_or(0, |brief| brief.required_suggestion_kinds.len()),
		..ProactiveBriefJobMetrics::default()
	};
	let mut suggestion_kinds = BTreeSet::new();

	for brief in &answer.proactive_briefs {
		accumulate_proactive_brief_metrics(brief, &mut metrics, &mut suggestion_kinds);
	}

	let covered_required_suggestion_kind_count = job.proactive_brief.as_ref().map_or(0, |brief| {
		brief
			.required_suggestion_kinds
			.iter()
			.filter(|kind| suggestion_kinds.contains(*kind))
			.count()
	});

	metrics.covered_required_suggestion_kind_count = covered_required_suggestion_kind_count;
	metrics.missing_required_suggestion_kind_count = metrics
		.required_suggestion_kind_count
		.saturating_sub(covered_required_suggestion_kind_count);
	metrics.evidence_ref_coverage =
		ratio(metrics.evidence_ref_suggestion_count, metrics.evidence_ref_required_count);
	metrics.freshness_coverage = ratio(metrics.freshness_marker_count, metrics.suggestion_count);
	metrics.action_rationale_coverage =
		ratio(metrics.action_rationale_count, metrics.suggestion_count);

	Some(metrics)
}

fn accumulate_proactive_brief_metrics(
	brief: &ProactiveBriefArtifact,
	metrics: &mut ProactiveBriefJobMetrics,
	suggestion_kinds: &mut BTreeSet<String>,
) {
	metrics.source_trace_selected_count += brief.source_trace.selected_source_refs.len();
	metrics.source_trace_dropped_count += brief.source_trace.dropped_source_refs.len();
	metrics.source_trace_stale_count += brief.source_trace.stale_source_refs.len();
	metrics.source_trace_superseded_count += brief.source_trace.superseded_source_refs.len();
	metrics.source_trace_tombstone_count += brief.source_trace.tombstone_source_refs.len();

	let non_current_refs = memory_summary_non_current_trace_refs(&brief.source_trace);
	let tombstone_refs = proactive_tombstone_trace_refs_impl(&brief.source_trace);

	for suggestion in &brief.suggestions {
		metrics.suggestion_count += 1;
		metrics.evidence_ref_required_count += 1;

		suggestion_kinds.insert(suggestion.suggestion_kind.clone());

		if suggestion.evidence_refs.is_empty() {
			metrics.untraced_suggestion_count += 1;
		} else {
			metrics.evidence_ref_suggestion_count += 1;
		}
		if proactive_suggestion_has_freshness(suggestion) {
			metrics.freshness_marker_count += 1;
		}
		if proactive_suggestion_has_action_rationale(suggestion) {
			metrics.action_rationale_count += 1;
		}

		accumulate_proactive_action_decision(suggestion.action.decision.as_str(), metrics);

		if suggestion.freshness.status == "current" {
			metrics.current_suggestion_count += 1;
		} else {
			metrics.non_current_suggestion_count += 1;
		}
		if proactive_suggestion_is_stale_warning(suggestion) {
			metrics.stale_warning_count += 1;
		}
		if proactive_suggestion_is_invalid_current(suggestion, &non_current_refs) {
			metrics.invalid_current_suggestion_count += 1;
		}
		if proactive_suggestion_is_unsupported_current(suggestion) {
			metrics.unsupported_current_suggestion_count += 1;
		}
		if proactive_suggestion_is_tombstone_violation(suggestion, &tombstone_refs) {
			metrics.tombstone_violation_count += 1;
		}
	}
}

pub(super) fn proactive_tombstone_trace_refs_impl(
	trace: &MemorySummarySourceTrace,
) -> BTreeSet<&str> {
	trace.tombstone_source_refs.iter().map(|item| item.evidence_id.as_str()).collect()
}

fn accumulate_proactive_action_decision(decision: &str, metrics: &mut ProactiveBriefJobMetrics) {
	match decision {
		"recommend" => metrics.recommended_count += 1,
		"defer" => metrics.deferred_count += 1,
		"reject" => metrics.rejected_count += 1,
		_ => {},
	}
}

fn proactive_suggestion_has_freshness(suggestion: &ProactiveSuggestion) -> bool {
	if suggestion.freshness.status.trim().is_empty() {
		return false;
	}

	match suggestion.freshness.status.as_str() {
		"superseded" => !suggestion.freshness.superseded_by.is_empty(),
		"tombstoned" => !suggestion.freshness.tombstone_refs.is_empty(),
		_ => true,
	}
}

fn proactive_suggestion_has_action_rationale(suggestion: &ProactiveSuggestion) -> bool {
	!suggestion.action.decision.trim().is_empty()
		&& !suggestion.action.reason_code.trim().is_empty()
		&& !suggestion.action.reason.trim().is_empty()
}

fn proactive_suggestion_is_stale_warning(suggestion: &ProactiveSuggestion) -> bool {
	matches!(
		suggestion.suggestion_kind.as_str(),
		"stale_decision_audit" | "stale_plan_preference_warning"
	) && suggestion.freshness.status != "current"
}

fn proactive_suggestion_is_invalid_current(
	suggestion: &ProactiveSuggestion,
	non_current_refs: &BTreeSet<&str>,
) -> bool {
	suggestion.freshness.status == "current"
		&& (!suggestion.freshness.superseded_by.is_empty()
			|| !suggestion.freshness.tombstone_refs.is_empty()
			|| suggestion
				.evidence_refs
				.iter()
				.any(|evidence_id| non_current_refs.contains(evidence_id.as_str())))
}

fn proactive_suggestion_is_unsupported_current(suggestion: &ProactiveSuggestion) -> bool {
	!suggestion.unsupported_claim_flags.is_empty()
		&& (suggestion.action.decision == "recommend" || suggestion.freshness.status == "current")
}

fn proactive_suggestion_is_tombstone_violation(
	suggestion: &ProactiveSuggestion,
	tombstone_refs: &BTreeSet<&str>,
) -> bool {
	suggestion.freshness.status == "current"
		&& (!suggestion.freshness.tombstone_refs.is_empty()
			|| suggestion
				.evidence_refs
				.iter()
				.any(|evidence_id| tombstone_refs.contains(evidence_id.as_str())))
}

pub(super) fn unsupported_proactive_suggestions_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Vec<UnsupportedClaimReport> {
	answer
		.proactive_briefs
		.iter()
		.flat_map(|brief| {
			brief.suggestions.iter().filter_map(|suggestion| {
				if suggestion.evidence_refs.is_empty() {
					return Some(proactive_unsupported_claim_report(
						job,
						brief,
						suggestion,
						"proactive suggestion has no evidence refs",
					));
				}
				if proactive_suggestion_is_unsupported_current(suggestion) {
					return Some(proactive_unsupported_claim_report(
						job,
						brief,
						suggestion,
						"unsupported proactive claim is still recommended or marked current",
					));
				}

				None
			})
		})
		.collect()
}

fn proactive_unsupported_claim_report(
	job: &RealWorldJob,
	brief: &ProactiveBriefArtifact,
	suggestion: &ProactiveSuggestion,
	reason: &str,
) -> UnsupportedClaimReport {
	UnsupportedClaimReport {
		suite_id: job.suite.clone(),
		job_id: job.job_id.clone(),
		claim_id: Some(format!("{}:{}", brief.brief_id, suggestion.suggestion_id)),
		claim_text: bounded_text(suggestion.body.as_str(), 240),
		reason: reason.to_string(),
		evidence_ids: suggestion.evidence_refs.clone(),
	}
}
