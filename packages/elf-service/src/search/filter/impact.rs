use std::{cmp::Ordering, collections::HashMap};

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::search::{
	ChunkCandidate, NoteMeta, SEARCH_FILTER_IMPACT_SCHEMA_V1, filter::parser::SearchFilter,
};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SearchFilterImpact {
	pub(crate) requested_candidate_k: u32,
	pub(crate) effective_candidate_k: u32,
	pub(crate) candidate_count_pre: usize,
	pub(crate) candidate_count_post: usize,
	pub(crate) dropped_total: usize,
	pub(crate) top_drop_reasons: Vec<SearchFilterDropReason>,
	pub(crate) filter: Value,
}
impl SearchFilterImpact {
	pub(crate) fn from_eval(
		filter: &SearchFilter,
		note_candidates: &[ChunkCandidate],
		note_meta: &HashMap<Uuid, NoteMeta>,
		requested_candidate_k: u32,
		effective_candidate_k: u32,
	) -> Self {
		let pre = note_candidates.len();
		let mut kept: Vec<ChunkCandidate> = Vec::new();
		let mut dropped_reason_counts: HashMap<String, usize> = HashMap::new();

		for candidate in note_candidates {
			let Some(note) = note_meta.get(&candidate.note_id) else {
				dropped_reason_counts
					.entry("note_meta_missing".to_string())
					.and_modify(|count| *count += 1)
					.or_insert(1);

				continue;
			};
			let (keep, reason) = filter.evaluate(note);

			if keep {
				kept.push(candidate.clone());
			} else {
				dropped_reason_counts
					.entry(reason.unwrap_or_else(|| "filter.no_match".to_string()))
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
		}

		let mut top_drop_reasons: Vec<_> = dropped_reason_counts
			.into_iter()
			.map(|(reason, count)| SearchFilterDropReason { reason, count })
			.collect();

		top_drop_reasons.sort_by(|a, b| match b.count.cmp(&a.count) {
			Ordering::Equal => a.reason.cmp(&b.reason),
			other => other,
		});
		top_drop_reasons.truncate(5);

		let post = kept.len();

		Self {
			requested_candidate_k,
			effective_candidate_k,
			candidate_count_pre: pre,
			candidate_count_post: post,
			dropped_total: pre.saturating_sub(post),
			top_drop_reasons,
			filter: filter.as_value(),
		}
	}

	pub(crate) fn to_stage_payload(&self) -> Value {
		serde_json::json!({
			"schema": SEARCH_FILTER_IMPACT_SCHEMA_V1,
			"requested_candidate_k": self.requested_candidate_k,
			"effective_candidate_k": self.effective_candidate_k,
			"candidate_count_pre": self.candidate_count_pre,
			"candidate_count_post": self.candidate_count_post,
			"dropped_total": self.dropped_total,
			"top_drop_reasons": self.top_drop_reasons,
			"filter": self.filter,
		})
	}
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SearchFilterDropReason {
	pub(crate) reason: String,
	pub(crate) count: usize,
}
