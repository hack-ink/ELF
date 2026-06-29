use super::super::*;

impl ElfService {
	pub(in crate::search) async fn build_snippet_items(
		&self,
		filtered_candidates: &[ChunkCandidate],
		note_meta: &HashMap<Uuid, NoteMeta>,
	) -> Result<Vec<ChunkSnippet>> {
		if filtered_candidates.is_empty() {
			return Ok(Vec::new());
		}

		let pairs = ranking::collect_neighbor_pairs(filtered_candidates);
		let chunk_rows = fetch_chunks_by_pair(&self.db.pool, &pairs).await?;
		let mut chunk_by_id = HashMap::new();
		let mut chunk_by_note_index = HashMap::new();

		for row in chunk_rows {
			chunk_by_note_index.insert((row.note_id, row.chunk_index), row.clone());
			chunk_by_id.insert(row.chunk_id, row);
		}

		let mut items = Vec::new();

		for candidate in filtered_candidates {
			let Some(chunk_row) = chunk_by_id.get(&candidate.chunk_id) else {
				tracing::warn!(
					chunk_id = %candidate.chunk_id,
					"Chunk metadata missing for candidate."
				);

				continue;
			};
			let snippet = ranking::stitch_snippet(
				candidate.note_id,
				chunk_row.chunk_index,
				&chunk_by_note_index,
			);

			if snippet.is_empty() {
				continue;
			}

			let Some(note) = note_meta.get(&candidate.note_id) else { continue };
			let chunk = ChunkMeta {
				chunk_id: chunk_row.chunk_id,
				chunk_index: chunk_row.chunk_index,
				start_offset: chunk_row.start_offset,
				end_offset: chunk_row.end_offset,
			};

			items.push(ChunkSnippet {
				note: note.clone(),
				chunk,
				snippet,
				retrieval_rank: candidate.retrieval_rank,
				retrieval_score: candidate.retrieval_score,
			});
		}

		Ok(items)
	}

	pub(in crate::search) async fn rerank_snippet_items(
		&self,
		query: &str,
		snippet_items: &[ChunkSnippet],
		cache_cfg: &SearchCache,
		now: OffsetDateTime,
	) -> Result<Vec<f32>> {
		if snippet_items.is_empty() {
			return Ok(Vec::new());
		}

		let (cache_candidates, signature) = Self::build_rerank_cache_signature(snippet_items);
		let mut cache_key: Option<String> = None;
		let mut cached_scores: Option<Vec<f32>> = None;

		if cache_cfg.enabled {
			match ranking::build_rerank_cache_key(
				query,
				self.cfg.providers.rerank.provider_id.as_str(),
				self.cfg.providers.rerank.model.as_str(),
				&signature,
			) {
				Ok(key) => {
					cache_key = Some(key.clone());
					cached_scores = self
						.read_rerank_cache_scores(&key, cache_candidates.as_slice(), cache_cfg, now)
						.await;
				},
				Err(err) => {
					tracing::warn!(
						error = %err,
						cache_kind = CacheKind::Rerank.as_str(),
						"Cache key build failed."
					);
				},
			}
		}

		if let Some(scores) = cached_scores {
			return Ok(scores);
		}

		let docs: Vec<String> = snippet_items.iter().map(|item| item.snippet.clone()).collect();
		let scores = self.providers.rerank.rerank(&self.cfg.providers.rerank, query, &docs).await?;

		if scores.len() != snippet_items.len() {
			return Err(crate::Error::Provider {
				message: "Rerank provider returned mismatched score count.".to_string(),
			});
		}
		if cache_cfg.enabled
			&& let Some(key) = cache_key.as_ref()
			&& !cache_candidates.is_empty()
		{
			self.store_rerank_cache_scores(
				key,
				cache_candidates.as_slice(),
				scores.as_slice(),
				cache_cfg,
			)
			.await;
		}

		Ok(scores)
	}

	pub(in crate::search) fn build_rerank_cache_signature(
		snippet_items: &[ChunkSnippet],
	) -> (Vec<RerankCacheCandidate>, Vec<(Uuid, OffsetDateTime)>) {
		let candidates: Vec<RerankCacheCandidate> = snippet_items
			.iter()
			.map(|item| RerankCacheCandidate {
				chunk_id: item.chunk.chunk_id,
				updated_at: item.note.updated_at,
			})
			.collect();
		let signature: Vec<(Uuid, OffsetDateTime)> =
			candidates.iter().map(|candidate| (candidate.chunk_id, candidate.updated_at)).collect();

		(candidates, signature)
	}

	pub(in crate::search) async fn read_rerank_cache_scores(
		&self,
		key: &str,
		cache_candidates: &[RerankCacheCandidate],
		cache_cfg: &SearchCache,
		now: OffsetDateTime,
	) -> Option<Vec<f32>> {
		match fetch_cache_payload(&self.db.pool, CacheKind::Rerank, key, now).await {
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

				match store_cache_payload(
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
