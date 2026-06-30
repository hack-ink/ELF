use crate::{
	Error,
	search::{
		CacheKind, ChunkSnippet, ElfService, OffsetDateTime, RerankCacheCandidate, Result,
		SearchCache, Uuid, ranking,
	},
};

impl ElfService {
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
			return Err(Error::Provider {
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
}
