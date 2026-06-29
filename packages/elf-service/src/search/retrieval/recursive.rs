use crate::search::{
	ChunkCandidate, Condition, ElfService, HashMap, HashSet, QueryEmbedding,
	RecursiveRetrievalArgs, RecursiveRetrievalResult, Result, VecDeque, ranking, slice,
};

impl ElfService {
	pub(in crate::search::retrieval) async fn run_recursive_retrieval(
		&self,
		args: RecursiveRetrievalArgs<'_>,
	) -> Result<RecursiveRetrievalResult> {
		let recursive_config = &self.cfg.search.recursive;
		let mut result = RecursiveRetrievalResult {
			enabled: recursive_config.enabled
				&& args.retrieval_sources_policy.recursive_weight > 0.0,
			..Default::default()
		};

		if !result.enabled {
			result.stop_reason = Some("disabled".to_string());

			return Ok(result);
		}
		if args.query_vec.is_empty() {
			result.stop_reason = Some("missing_query_vector".to_string());

			return Ok(result);
		}

		let mut seed_scopes = HashSet::<String>::new();

		for candidate in args.seed_candidates {
			if let Some(scope) = candidate.scope.as_deref()
				&& !scope.trim().is_empty()
			{
				seed_scopes.insert(scope.to_string());
			}
		}

		result.scopes_seeded = seed_scopes.len();
		result.candidates_before = args.seed_candidates.len();

		if seed_scopes.is_empty() {
			result.stop_reason = Some("no_scope_seed".to_string());

			return Ok(result);
		}

		let max_depth = recursive_config.max_depth;
		let max_children_per_node =
			usize::try_from(recursive_config.max_children_per_node).unwrap_or(usize::MAX);
		let max_nodes_per_scope =
			usize::try_from(recursive_config.max_nodes_per_scope).unwrap_or(usize::MAX);
		let max_total_nodes =
			usize::try_from(recursive_config.max_total_nodes).unwrap_or(usize::MAX);
		let child_query_embedding =
			QueryEmbedding { text: args.query.to_string(), vector: args.query_vec.to_vec() };
		let per_query_candidate_k =
			args.candidate_k.min(recursive_config.max_nodes_per_scope).max(1);
		let (candidates, queried_scopes, rounds_executed, stop_reason) = self
			.collect_recursive_candidates(
				&args,
				seed_scopes,
				child_query_embedding,
				max_depth,
				max_children_per_node,
				max_nodes_per_scope,
				max_total_nodes,
				per_query_candidate_k,
				self.cfg.search.prefilter.max_candidates,
			)
			.await?;

		result.scopes_queried = queried_scopes;
		result.rounds_executed = rounds_executed;
		result.total_queries = rounds_executed;
		result.candidates = candidates;
		result.candidates_added = result.candidates.len();
		result.candidates_after = result.candidates_before + result.candidates_added;
		result.stop_reason = stop_reason.or(Some("converged".to_string()));

		Ok(result)
	}

	#[allow(clippy::too_many_arguments)]
	async fn collect_recursive_candidates(
		&self,
		args: &RecursiveRetrievalArgs<'_>,
		seed_scopes: HashSet<String>,
		child_query_embedding: QueryEmbedding,
		max_depth: u32,
		max_children_per_node: usize,
		max_nodes_per_scope: usize,
		max_total_nodes: usize,
		per_query_candidate_k: u32,
		prefilter_max_candidates: u32,
	) -> Result<(Vec<ChunkCandidate>, usize, u32, Option<String>)> {
		let mut queued_scopes: VecDeque<(String, u32)> = VecDeque::new();
		let mut discovered_scopes = seed_scopes.clone();
		let mut recursion_candidates = Vec::<ChunkCandidate>::new();
		let mut seen_chunks =
			args.seed_candidates.iter().map(|candidate| candidate.chunk_id).collect::<HashSet<_>>();
		let mut scope_counts: HashMap<String, u32> = HashMap::new();
		let mut queried_scopes = 0_usize;
		let mut rounds_executed = 0_u32;
		let mut stop_reason: Option<String> = None;

		for scope in seed_scopes {
			queued_scopes.push_back((scope, 1));
		}

		while let Some((scope, depth)) = queued_scopes.pop_front() {
			if depth > max_depth {
				stop_reason = Some("max_depth".to_string());

				break;
			}

			queried_scopes = queried_scopes.saturating_add(1);
			rounds_executed = rounds_executed.saturating_add(1);

			let mut scoped_filter = args.filter.clone();

			scoped_filter.must.push(Condition::matches("scope", scope.clone()));

			let recursive_points = self
				.run_fusion_query(
					slice::from_ref(&child_query_embedding),
					&scoped_filter,
					per_query_candidate_k,
				)
				.await?;
			let scope_query_limit = per_query_candidate_k.min(max_nodes_per_scope as u32);
			let recursive_candidates_for_scope = ranking::collect_chunk_candidates(
				&recursive_points,
				prefilter_max_candidates.min(scope_query_limit),
				scope_query_limit,
			);
			let mut child_scopes = HashSet::<String>::new();

			for mut candidate in recursive_candidates_for_scope {
				if recursion_candidates.len() >= max_total_nodes {
					stop_reason = Some("max_total_nodes".to_string());

					break;
				}

				let scope_key = candidate.scope.clone().unwrap_or_else(|| scope.clone());
				let scope_count = scope_counts.entry(scope_key.clone()).or_default();

				if (*scope_count as usize) >= max_nodes_per_scope {
					continue;
				}
				if !seen_chunks.insert(candidate.chunk_id) {
					continue;
				}

				*scope_count = scope_count.saturating_add(1);
				candidate.scope = Some(scope_key.clone());

				recursion_candidates.push(candidate);

				if depth < max_depth
					&& child_scopes.len() < max_children_per_node
					&& !scope_key.is_empty()
					&& discovered_scopes.insert(scope_key.clone())
				{
					child_scopes.insert(scope_key.clone());
					queued_scopes.push_back((scope_key.clone(), depth.saturating_add(1)));
				}
			}

			if stop_reason.is_some() {
				break;
			}
		}

		Ok((recursion_candidates, queried_scopes, rounds_executed, stop_reason))
	}
}
