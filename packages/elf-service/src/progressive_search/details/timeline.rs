use std::{cmp::Ordering, collections::BTreeMap};

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Result,
	progressive_search::types::{
		SearchIndexItem, SearchSessionItemRecord, SearchTimelineGroup, SearchTimelineResponse,
	},
};

pub(crate) fn build_timeline_by_day(
	search_session_id: Uuid,
	expires_at: OffsetDateTime,
	items: &[SearchSessionItemRecord],
) -> Result<SearchTimelineResponse> {
	let mut grouped: BTreeMap<String, Vec<SearchIndexItem>> = BTreeMap::new();

	for item in items {
		let date = item.updated_at.date().to_string();

		grouped.entry(date).or_default().push(item.to_index_item());
	}

	let mut groups = Vec::with_capacity(grouped.len());

	for (date, mut items) in grouped.into_iter().rev() {
		items.sort_by(|a, b| {
			b.updated_at
				.cmp(&a.updated_at)
				.then_with(|| b.final_score.partial_cmp(&a.final_score).unwrap_or(Ordering::Equal))
		});
		groups.push(SearchTimelineGroup { date, items });
	}

	Ok(SearchTimelineResponse { search_session_id, expires_at, groups })
}
