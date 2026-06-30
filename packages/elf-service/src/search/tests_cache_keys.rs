use crate::search::{
	OffsetDateTime, RerankCacheCandidate, RerankCacheItem, RerankCachePayload, Uuid, ranking,
};

#[test]
fn expansion_cache_key_changes_with_max_queries() {
	let key_a = ranking::build_expansion_cache_key("alpha", 4, true, "llm", "model", 0.1_f32)
		.expect("Expected cache key.");
	let key_b = ranking::build_expansion_cache_key("alpha", 5, true, "llm", "model", 0.1_f32)
		.expect("Expected cache key.");

	assert_ne!(key_a, key_b);
}

#[test]
fn rerank_cache_key_changes_with_updated_at() {
	let ts_a = OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp.");
	let ts_b = OffsetDateTime::from_unix_timestamp(2).expect("Valid timestamp.");
	let chunk_id = Uuid::new_v4();
	let key_a = ranking::build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_a)])
		.expect("Expected cache key.");
	let key_b = ranking::build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_b)])
		.expect("Expected cache key.");

	assert_ne!(key_a, key_b);
}

#[test]
fn rerank_cache_payload_rejects_mismatched_counts() {
	let payload = RerankCachePayload {
		items: vec![RerankCacheItem {
			chunk_id: Uuid::new_v4(),
			updated_at: OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
			score: 0.5,
		}],
	};
	let candidates = vec![RerankCacheCandidate {
		chunk_id: Uuid::new_v4(),
		updated_at: OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
	}];

	assert!(ranking::build_cached_scores(&payload, &candidates).is_none());
}

#[test]
fn cache_key_prefix_is_stable() {
	let prefix = ranking::cache_key_prefix("abcd1234efgh5678");

	assert_eq!(prefix, "abcd1234efgh");
}
