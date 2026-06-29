use super::super::*;

impl ElfService {
	pub(in crate::search::retrieval) async fn expand_queries(&self, query: &str) -> Vec<String> {
		let cfg = &self.cfg.search.expansion;
		let cache_cfg = &self.cfg.search.cache;
		let now = OffsetDateTime::now_utc();
		let cache_key = if cache_cfg.enabled {
			match ranking::build_expansion_cache_key(
				query,
				cfg.max_queries,
				cfg.include_original,
				self.cfg.providers.llm_extractor.provider_id.as_str(),
				self.cfg.providers.llm_extractor.model.as_str(),
				self.cfg.providers.llm_extractor.temperature,
			) {
				Ok(key) => Some(key),
				Err(err) => {
					tracing::warn!(
						error = %err,
						cache_kind = CacheKind::Expansion.as_str(),
						"Cache key build failed."
					);

					None
				},
			}
		} else {
			None
		};

		if let Some(key) = cache_key.as_ref()
			&& let Some(queries) = self.read_expansion_cache_queries(key, cache_cfg, now).await
		{
			return queries;
		}

		let messages =
			ranking::build_expansion_messages(query, cfg.max_queries, cfg.include_original);
		let raw = match self
			.providers
			.extractor
			.extract(&self.cfg.providers.llm_extractor, &messages)
			.await
		{
			Ok(value) => value,
			Err(err) => {
				tracing::warn!(error = %err, "Query expansion failed; falling back to original query.");

				return vec![query.to_string()];
			},
		};
		let parsed: ExpansionOutput = match serde_json::from_value(raw) {
			Ok(value) => value,
			Err(err) => {
				tracing::warn!(error = %err, "Query expansion returned invalid JSON; falling back to original query.");

				return vec![query.to_string()];
			},
		};
		let normalized = ranking::normalize_queries(
			parsed.queries,
			query,
			cfg.include_original,
			cfg.max_queries,
		);
		let result = if normalized.is_empty() { vec![query.to_string()] } else { normalized };

		if let Some(key) = cache_key {
			self.store_expansion_cache_queries(&key, &result, cache_cfg).await;
		}

		result
	}

	async fn read_expansion_cache_queries(
		&self,
		key: &str,
		cache_cfg: &SearchCache,
		now: OffsetDateTime,
	) -> Option<Vec<String>> {
		match fetch_cache_payload(&self.db.pool, CacheKind::Expansion, key, now).await {
			Ok(Some(payload)) => {
				tracing::info!(
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					hit = true,
					payload_size = payload.size_bytes,
					ttl_days = cache_cfg.expansion_ttl_days,
					"Cache hit."
				);

				let cached: ExpansionCachePayload = match serde_json::from_value(payload.value) {
					Ok(value) => value,
					Err(err) => {
						tracing::warn!(
							error = %err,
							cache_kind = CacheKind::Expansion.as_str(),
							cache_key_prefix = ranking::cache_key_prefix(key),
							"Cache payload decode failed."
						);

						ExpansionCachePayload { queries: Vec::new() }
					},
				};

				(!cached.queries.is_empty()).then_some(cached.queries)
			},
			Ok(None) => {
				tracing::info!(
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					hit = false,
					payload_size = 0_u64,
					ttl_days = cache_cfg.expansion_ttl_days,
					"Cache miss."
				);

				None
			},
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache read failed."
				);

				None
			},
		}
	}

	async fn store_expansion_cache_queries(
		&self,
		key: &str,
		queries: &[String],
		cache_cfg: &SearchCache,
	) {
		let payload = ExpansionCachePayload { queries: queries.to_vec() };
		let payload_json = match serde_json::to_value(&payload) {
			Ok(value) => value,
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache payload encode failed."
				);

				return;
			},
		};
		let stored_at = OffsetDateTime::now_utc();
		let expires_at = stored_at + Duration::days(cache_cfg.expansion_ttl_days);

		match store_cache_payload(
			&self.db.pool,
			CacheKind::Expansion,
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
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					hit = false,
					payload_size,
					ttl_days = cache_cfg.expansion_ttl_days,
					"Cache stored."
				);
			},
			Ok(None) => {
				tracing::warn!(
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					hit = false,
					payload_size = 0_u64,
					ttl_days = cache_cfg.expansion_ttl_days,
					"Cache payload skipped due to size."
				);
			},
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache write failed."
				);
			},
		}
	}
}
