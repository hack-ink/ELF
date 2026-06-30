use crate::search::{
	self, CacheKind, Duration, ElfService, OffsetDateTime, RerankCacheCandidate, RerankCacheItem,
	RerankCachePayload, SearchCache, ranking,
};

impl ElfService {
	pub(in crate::search) async fn read_rerank_cache_scores(
		&self,
		key: &str,
		cache_candidates: &[RerankCacheCandidate],
		cache_cfg: &SearchCache,
		now: OffsetDateTime,
	) -> Option<Vec<f32>> {
		match search::fetch_cache_payload(&self.db.pool, CacheKind::Rerank, key, now).await {
			Ok(Some(payload)) => {
				let decoded: RerankCachePayload = match serde_json::from_value(payload.value) {
					Ok(value) => value,
					Err(err) => {
						tracing::warn!(
							error = %err,
							cache_kind = CacheKind::Rerank.as_str(),
							cache_key_prefix = ranking::cache_key_prefix(key),
							"Cache payload decode failed."
						);

						RerankCachePayload { items: Vec::new() }
					},
				};

				if let Some(scores) = ranking::build_cached_scores(&decoded, cache_candidates) {
					tracing::info!(
						cache_kind = CacheKind::Rerank.as_str(),
						cache_key_prefix = ranking::cache_key_prefix(key),
						hit = true,
						payload_size = payload.size_bytes,
						ttl_days = cache_cfg.rerank_ttl_days,
						"Cache hit."
					);

					Some(scores)
				} else {
					tracing::warn!(
						cache_kind = CacheKind::Rerank.as_str(),
						cache_key_prefix = ranking::cache_key_prefix(key),
						hit = false,
						payload_size = payload.size_bytes,
						ttl_days = cache_cfg.rerank_ttl_days,
						"Cache payload did not match candidates."
					);

					None
				}
			},
			Ok(None) => {
				tracing::info!(
					cache_kind = CacheKind::Rerank.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					hit = false,
					payload_size = 0_u64,
					ttl_days = cache_cfg.rerank_ttl_days,
					"Cache miss."
				);

				None
			},
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Rerank.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache read failed."
				);

				None
			},
		}
	}

	pub(in crate::search) async fn store_rerank_cache_scores(
		&self,
		key: &str,
		cache_candidates: &[RerankCacheCandidate],
		scores: &[f32],
		cache_cfg: &SearchCache,
	) {
		let payload = RerankCachePayload {
			items: cache_candidates
				.iter()
				.zip(scores.iter())
				.map(|(candidate, score)| RerankCacheItem {
					chunk_id: candidate.chunk_id,
					updated_at: candidate.updated_at,
					score: *score,
				})
				.collect(),
		};

		match serde_json::to_value(&payload) {
			Ok(payload_json) => {
				let stored_at = OffsetDateTime::now_utc();
				let expires_at = stored_at + Duration::days(cache_cfg.rerank_ttl_days);

				match search::store_cache_payload(
					&self.db.pool,
					CacheKind::Rerank,
					key,
					payload_json,
					stored_at,
					expires_at,
					cache_cfg.max_payload_bytes,
				)
				.await
				{
					Ok(Some(payload_size)) => {
						tracing::info!(
							cache_kind = CacheKind::Rerank.as_str(),
							cache_key_prefix = ranking::cache_key_prefix(key),
							hit = false,
							payload_size,
							ttl_days = cache_cfg.rerank_ttl_days,
							"Cache stored."
						);
					},
					Ok(None) => {
						tracing::warn!(
							cache_kind = CacheKind::Rerank.as_str(),
							cache_key_prefix = ranking::cache_key_prefix(key),
							hit = false,
							payload_size = 0_u64,
							ttl_days = cache_cfg.rerank_ttl_days,
							"Cache payload skipped due to size."
						);
					},
					Err(err) => {
						tracing::warn!(
							error = %err,
							cache_kind = CacheKind::Rerank.as_str(),
							cache_key_prefix = ranking::cache_key_prefix(key),
							"Cache write failed."
						);
					},
				}
			},
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Rerank.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache payload encode failed."
				);
			},
		}
	}
}
