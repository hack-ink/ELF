use crate::{
	Error,
	search::{
		BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME, Document, ElfService, Filter, Fusion,
		PrefetchQueryBuilder, Query, QueryEmbedding, QueryPointsBuilder, Result, ScoredPoint,
		english_gate, ranking, slice,
	},
};

impl ElfService {
	pub(in crate::search) fn resolve_project_context_description<'a>(
		&'a self,
		tenant_id: &str,
		project_id: &str,
	) -> Option<&'a str> {
		let context = self.cfg.context.as_ref()?;
		let descriptions = context.project_descriptions.as_ref()?;
		let key = format!("{tenant_id}:{project_id}");
		let mut saw_non_english = false;

		if let Some(value) = descriptions.get(&key) {
			let trimmed = value.trim();

			if !trimmed.is_empty() {
				if !english_gate::is_english_natural_language(trimmed) {
					saw_non_english = true;
				} else {
					return Some(trimmed);
				}
			}
		}
		if let Some(value) = descriptions.get(project_id) {
			let trimmed = value.trim();

			if !trimmed.is_empty() {
				if !english_gate::is_english_natural_language(trimmed) {
					saw_non_english = true;
				} else {
					return Some(trimmed);
				}
			}
		}

		if saw_non_english {
			tracing::warn!(
				tenant_id = %tenant_id,
				project_id = %project_id,
				"Project context description is non-English. Skipping context."
			);
		}

		None
	}

	pub(in crate::search::retrieval) async fn embed_single_query(
		&self,
		query: &str,
		project_context_description: Option<&str>,
	) -> Result<Vec<f32>> {
		let input = ranking::build_dense_embedding_input(query, project_context_description);
		let embeddings = self
			.providers
			.embedding
			.embed(&self.cfg.providers.embedding, slice::from_ref(&input))
			.await?;
		let query_vec = embeddings.into_iter().next().ok_or_else(|| Error::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		})?;

		if query_vec.len() != self.cfg.storage.qdrant.vector_dim as usize {
			return Err(Error::Provider {
				message: "Embedding vector dimension mismatch.".to_string(),
			});
		}

		Ok(query_vec)
	}

	pub(in crate::search::retrieval) async fn embed_queries(
		&self,
		queries: &[String],
		original_query: &str,
		baseline_vector: Option<&Vec<f32>>,
		project_context_description: Option<&str>,
	) -> Result<Vec<QueryEmbedding>> {
		let mut extra_queries = Vec::new();
		let mut extra_inputs = Vec::new();

		for query in queries {
			if baseline_vector.is_some() && query == original_query {
				continue;
			}

			extra_queries.push(query.clone());
			extra_inputs
				.push(ranking::build_dense_embedding_input(query, project_context_description));
		}

		let mut embedded_iter = if extra_queries.is_empty() {
			Vec::new().into_iter()
		} else {
			let embedded = self
				.providers
				.embedding
				.embed(&self.cfg.providers.embedding, &extra_inputs)
				.await?;

			if embedded.len() != extra_queries.len() {
				return Err(Error::Provider {
					message: "Embedding provider returned mismatched vector count.".to_string(),
				});
			}

			embedded.into_iter()
		};
		let mut out = Vec::with_capacity(queries.len());

		for query in queries {
			let vector = if baseline_vector.is_some() && query == original_query {
				baseline_vector
					.ok_or_else(|| Error::Provider {
						message: "Embedding baseline vector is missing.".to_string(),
					})?
					.clone()
			} else {
				embedded_iter.next().ok_or_else(|| Error::Provider {
					message: "Embedding provider returned no vectors.".to_string(),
				})?
			};

			if vector.len() != self.cfg.storage.qdrant.vector_dim as usize {
				return Err(Error::Provider {
					message: "Embedding vector dimension mismatch.".to_string(),
				});
			}

			out.push(QueryEmbedding { text: query.clone(), vector });
		}

		Ok(out)
	}

	pub(in crate::search::retrieval) async fn run_fusion_query(
		&self,
		queries: &[QueryEmbedding],
		filter: &Filter,
		candidate_k: u32,
	) -> Result<Vec<ScoredPoint>> {
		let mut search = QueryPointsBuilder::new(self.qdrant.collection.clone());

		for query in queries {
			let dense_prefetch = PrefetchQueryBuilder::default()
				.query(Query::new_nearest(query.vector.clone()))
				.using(DENSE_VECTOR_NAME)
				.filter(filter.clone())
				.limit(candidate_k as u64);
			let bm25_prefetch = PrefetchQueryBuilder::default()
				.query(Query::new_nearest(Document::new(query.text.clone(), BM25_MODEL)))
				.using(BM25_VECTOR_NAME)
				.filter(filter.clone())
				.limit(candidate_k as u64);

			search = search.add_prefetch(dense_prefetch).add_prefetch(bm25_prefetch);
		}

		let search = search.with_payload(true).query(Fusion::Rrf).limit(candidate_k as u64);
		let response = self
			.qdrant
			.client
			.query(search)
			.await
			.map_err(|err| Error::Qdrant { message: err.to_string() })?;

		Ok(response.result)
	}
}
