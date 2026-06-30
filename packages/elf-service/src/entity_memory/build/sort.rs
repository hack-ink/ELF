use crate::entity_memory::types::EntityMemoryItem;

pub(in crate::entity_memory) fn sort_entity_memory_items(items: &mut [EntityMemoryItem]) {
	items.sort_by(|left, right| {
		read_bucket_rank(right.read_bucket.as_str())
			.cmp(&read_bucket_rank(left.read_bucket.as_str()))
			.then_with(|| {
				lifecycle_rank(right.lifecycle.as_str())
					.cmp(&lifecycle_rank(left.lifecycle.as_str()))
			})
			.then_with(|| right.updated_at.cmp(&left.updated_at))
			.then_with(|| left.source.cmp(&right.source))
	});
}

fn read_bucket_rank(bucket: &str) -> u8 {
	match bucket {
		"top_of_mind" => 1,
		_ => 0,
	}
}

fn lifecycle_rank(lifecycle: &str) -> u8 {
	match lifecycle {
		"current" => 3,
		"stale" => 2,
		"superseded" => 1,
		_ => 0,
	}
}
