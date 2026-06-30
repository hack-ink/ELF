use crate::entity_memory::types::{EntityMemoryItem, EntityMemorySummary};

pub(in crate::entity_memory) fn summarize_items(items: &[EntityMemoryItem]) -> EntityMemorySummary {
	let mut summary = EntityMemorySummary::default();

	for item in items {
		match item.lifecycle.as_str() {
			"current" => summary.current_count += 1,
			"stale" => summary.stale_count += 1,
			"superseded" => summary.superseded_count += 1,
			"tombstoned" => summary.tombstoned_count += 1,
			_ => {},
		}
		match item.read_bucket.as_str() {
			"top_of_mind" => summary.top_of_mind_count += 1,
			"background" => summary.background_count += 1,
			_ => {},
		}
		match item.source.as_str() {
			"core_block" => summary.core_block_count += 1,
			"archival_note" => summary.archival_note_count += 1,
			_ => {},
		}
	}

	summary
}
