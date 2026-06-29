use crate::search::{
	DynamicGateSummary, ElfService, ExpansionMode, FinishSearchArgs, MaybeDynamicSearchArgs,
	OffsetDateTime, QueryEmbedding, RecursiveRetrievalArgs, Result, RetrievalSourceCandidates,
	RetrievalSourceKind, SearchResponse, SearchRetrievalArgs, SearchRetrievalResult,
	StructuredFieldRetrievalArgs, StructuredFieldRetrievalResult, ranking,
};

impl ElfService {
	pub(in crate::search) async fn maybe_finish_dynamic_search(
		&self,
		args: MaybeDynamicSearchArgs<'_>,
	) -> Result<(Option<Vec<f32>>, Option<SearchResponse>, DynamicGateSummary)> {
		if !args.enabled {
			return Ok((None, None, DynamicGateSummary::default()));
		}

		let query_vec =
			self.embed_single_query(args.query, args.project_context_description).await?;
		let baseline_points = self
			.run_fusion_query(
				&[QueryEmbedding { text: args.query.to_string(), vector: query_vec.clone() }],
				args.filter,
				args.candidate_k,
			)
			.await?;
		let top_score = baseline_points.first().map(|point| point.score).unwrap_or(0.0);
		let fusion_candidates = ranking::collect_chunk_candidates(
			&baseline_points,
			self.cfg.search.prefilter.max_candidates,
			args.candidate_k,
		);
		let should_expand = ranking::should_expand_dynamic(
			baseline_points.len(),
			top_score,
			&self.cfg.search.dynamic,
		);
		let dynamic_gate = DynamicGateSummary {
			considered: true,
			should_expand: Some(should_expand),
			observed_candidates: Some(baseline_points.len() as u32),
			observed_top_score: Some(top_score),
		};

		if should_expand {
			return Ok((Some(query_vec), None, dynamic_gate));
		}

		let StructuredFieldRetrievalResult {
			candidates: structured_candidates,
			structured_matches,
		} = self
			.retrieve_structured_field_candidates(StructuredFieldRetrievalArgs {
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				allowed_scopes: args.allowed_scopes,
				query_vec: query_vec.as_slice(),
				candidate_k: args.candidate_k,
				now: OffsetDateTime::now_utc(),
			})
			.await?;
		let mut seed_candidates =
			Vec::with_capacity(fusion_candidates.len() + structured_candidates.len());

		seed_candidates.extend_from_slice(fusion_candidates.as_slice());
		seed_candidates.extend_from_slice(structured_candidates.as_slice());

		let recursive = self
			.run_recursive_retrieval(RecursiveRetrievalArgs {
				query: args.query,
				query_vec: query_vec.as_slice(),
				filter: args.filter,
				candidate_k: args.candidate_k,
				retrieval_sources_policy: args.retrieval_sources_policy,
				seed_candidates: seed_candidates.as_slice(),
			})
			.await?;
		let mut retrieval_sources = vec![
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::Fusion,
				candidates: fusion_candidates,
			},
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::StructuredField,
				candidates: structured_candidates,
			},
		];

		if recursive.enabled {
			retrieval_sources.push(RetrievalSourceCandidates {
				source: RetrievalSourceKind::Recursive,
				candidates: recursive.candidates.clone(),
			});
		}

		let merged_candidates = ranking::merge_retrieval_candidates(
			retrieval_sources,
			args.retrieval_sources_policy,
			args.candidate_k,
		);
		let response = self
			.finish_search(FinishSearchArgs {
				path: args.path,
				trace_id: args.trace_id,
				query: args.query,
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				token_id: args.token_id,
				read_profile: args.read_profile,
				allowed_scopes: args.allowed_scopes,
				expanded_queries: vec![args.query.to_string()],
				expansion_mode: ExpansionMode::Dynamic,
				candidates: merged_candidates,
				structured_matches,
				recursive_retrieval: Some(recursive),
				top_k: args.top_k,
				record_hits_enabled: args.record_hits_enabled,
				ranking_override: args.ranking_override.cloned(),
				payload_level: args.payload_level,
				filter: args.service_filter,
				requested_candidate_k: args.requested_candidate_k,
				effective_candidate_k: args.effective_candidate_k,
			})
			.await?;

		Ok((Some(query_vec), Some(response), dynamic_gate))
	}

	pub(in crate::search) async fn retrieve_search_candidates(
		&self,
		args: SearchRetrievalArgs<'_>,
	) -> Result<SearchRetrievalResult> {
		let queries = match args.expansion_mode {
			ExpansionMode::Off => vec![args.query.to_string()],
			ExpansionMode::Always | ExpansionMode::Dynamic => self.expand_queries(args.query).await,
		};
		let expanded_queries = queries.clone();
		let query_embeddings = self
			.embed_queries(
				queries.as_slice(),
				args.query,
				args.baseline_vector,
				args.project_context_description,
			)
			.await?;
		let fusion_points =
			self.run_fusion_query(&query_embeddings, args.filter, args.candidate_k).await?;
		let fusion_candidates = ranking::collect_chunk_candidates(
			&fusion_points,
			self.cfg.search.prefilter.max_candidates,
			args.candidate_k,
		);
		let original_query_vec = query_embeddings
			.iter()
			.find(|embedded| embedded.text == args.query)
			.map(|embedded| embedded.vector.clone())
			.unwrap_or_else(Vec::new);
		let original_query_vec = if original_query_vec.is_empty() {
			self.embed_single_query(args.query, args.project_context_description).await?
		} else {
			original_query_vec
		};
		let StructuredFieldRetrievalResult {
			candidates: structured_candidates,
			structured_matches,
		} = self
			.retrieve_structured_field_candidates(StructuredFieldRetrievalArgs {
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				allowed_scopes: args.allowed_scopes,
				query_vec: original_query_vec.as_slice(),
				candidate_k: args.candidate_k,
				now: OffsetDateTime::now_utc(),
			})
			.await?;
		let mut seed_candidates =
			Vec::with_capacity(fusion_candidates.len() + structured_candidates.len());

		seed_candidates.extend_from_slice(fusion_candidates.as_slice());
		seed_candidates.extend_from_slice(structured_candidates.as_slice());

		let recursive = self
			.run_recursive_retrieval(RecursiveRetrievalArgs {
				query: args.query,
				query_vec: original_query_vec.as_slice(),
				filter: args.filter,
				candidate_k: args.candidate_k,
				retrieval_sources_policy: args.retrieval_sources_policy,
				seed_candidates: seed_candidates.as_slice(),
			})
			.await?;
		let mut retrieval_sources = vec![
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::Fusion,
				candidates: fusion_candidates,
			},
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::StructuredField,
				candidates: structured_candidates,
			},
		];

		if recursive.enabled {
			retrieval_sources.push(RetrievalSourceCandidates {
				source: RetrievalSourceKind::Recursive,
				candidates: recursive.candidates.clone(),
			});
		}

		let merged_candidates = ranking::merge_retrieval_candidates(
			retrieval_sources,
			args.retrieval_sources_policy,
			args.candidate_k,
		);

		Ok(SearchRetrievalResult {
			expanded_queries,
			candidates: merged_candidates,
			structured_matches,
			recursive: Some(recursive),
		})
	}
}
