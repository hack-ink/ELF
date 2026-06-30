use std::collections::BTreeSet;

use crate::dreaming_review_queue::types::{DreamingReviewQueueItem, DreamingReviewQueueSummary};

pub(in crate::dreaming_review_queue) fn summarize_items(
	items: &[DreamingReviewQueueItem],
) -> DreamingReviewQueueSummary {
	let mut summary = DreamingReviewQueueSummary {
		item_count: items.len(),
		..DreamingReviewQueueSummary::default()
	};
	let mut variants = BTreeSet::new();

	for item in items {
		match item.review_state.as_str() {
			"proposed" => summary.proposed_count += 1,
			"approved" => summary.approved_count += 1,
			"applied" => summary.applied_count += 1,
			"rejected" => summary.discarded_count += 1,
			"archived" => summary.deferred_count += 1,
			_ => {},
		}

		if item.policy.high_impact {
			summary.high_impact_count += 1;
		}
		if item.policy.source_mutation_requested {
			summary.source_mutation_requested_count += 1;
		}
		if item.policy.auto_apply_candidate {
			summary.auto_apply_candidate_count += 1;
		}
		if item.policy.auto_apply_allowed {
			summary.auto_apply_allowed_count += 1;
		}

		variants.insert(item.queue_variant.as_str());
	}

	summary.variant_count = variants.len();

	summary
}
