const DEFAULT_QUEUE_LIMIT: u32 = 50;
const MAX_QUEUE_LIMIT: u32 = 200;

pub(in crate::dreaming_review_queue) fn available_review_actions(
	review_state: &str,
	manual_apply_allowed: bool,
) -> Vec<String> {
	let actions = match review_state {
		"proposed" => &["approve", "defer", "discard"][..],
		"approved" if manual_apply_allowed => &["apply", "defer", "discard"][..],
		"approved" => &["defer", "discard"][..],
		_ => &[][..],
	};

	actions.iter().map(|action| (*action).to_string()).collect()
}

#[allow(clippy::too_many_arguments)]
pub(in crate::dreaming_review_queue) fn policy_reason(
	source_mutation_requested: bool,
	high_impact: bool,
	has_unsupported_claims: bool,
	has_review_markers: bool,
	auto_apply_candidate: bool,
	auto_apply_allowed: bool,
	manual_apply_allowed: bool,
) -> String {
	if source_mutation_requested {
		return "source mutation is requested, so the proposal cannot be applied by the queue"
			.to_string();
	}
	if has_unsupported_claims || has_review_markers {
		return "lint or review markers require explicit reviewer inspection".to_string();
	}
	if auto_apply_allowed {
		return "approved low-risk derived organization proposal satisfies auto-apply policy"
			.to_string();
	}
	if manual_apply_allowed {
		return "approved review-gated proposal may be manually applied to a derived target"
			.to_string();
	}
	if high_impact {
		return "high-impact memory, graph, or correction proposal requires approval before apply"
			.to_string();
	}

	if auto_apply_candidate {
		return "low-risk derived organization proposal is a candidate after reviewer approval"
			.to_string();
	}

	"proposal remains reviewable derived output".to_string()
}

pub(in crate::dreaming_review_queue) fn bounded_queue_limit(limit: Option<u32>) -> i64 {
	i64::from(limit.unwrap_or(DEFAULT_QUEUE_LIMIT).clamp(1, MAX_QUEUE_LIMIT))
}
