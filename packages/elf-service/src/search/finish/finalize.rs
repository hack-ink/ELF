use crate::search::{
	BuildTraceArgs, ElfService, FinishSearchArgs, FinishSearchScoringResult, OffsetDateTime,
	RawSearchPath, Result, SearchResponse, Uuid, ranking,
};

impl ElfService {
	pub(in crate::search) async fn finish_search(
		&self,
		args: FinishSearchArgs<'_>,
	) -> Result<SearchResponse> {
		let now = OffsetDateTime::now_utc();
		let candidate_count = args.candidates.len();
		let candidate_note_ids: Vec<Uuid> =
			args.candidates.iter().map(|candidate| candidate.note_id).collect();
		let policies = self.resolve_finish_search_policies(args.ranking_override.as_ref())?;
		let note_meta = self
			.fetch_note_meta_for_candidates(
				args.tenant_id,
				args.project_id,
				args.agent_id,
				args.allowed_scopes,
				candidate_note_ids.as_slice(),
				now,
			)
			.await?;
		let scoring = self
			.build_finish_search_scoring(
				args.query,
				args.candidates,
				&note_meta,
				&policies,
				args.top_k,
				candidate_count,
				args.filter,
				args.requested_candidate_k,
				args.effective_candidate_k,
				now,
				args.path == RawSearchPath::Quick,
			)
			.await?;
		let FinishSearchScoringResult {
			query_tokens,
			filtered_candidates,
			scored_count,
			snippet_count,
			filtered_candidate_count,
			filter_impact,
			mut trace_candidates,
			fused_results,
			selected_results,
			diversity_decisions,
			selected_count,
		} = scoring;
		let relation_contexts = self
			.build_relation_context_for_selected_results(
				&selected_results,
				args.tenant_id,
				args.project_id,
				args.agent_id,
				args.allowed_scopes,
				now,
			)
			.await?;

		ranking::attach_diversity_decisions_to_trace_candidates(
			&mut trace_candidates,
			&diversity_decisions,
		);

		self.record_hits_if_enabled(args.record_hits_enabled, args.query, &selected_results, now)
			.await?;

		let (items, trajectory_summary) = self
			.build_items_and_write_trace(BuildTraceArgs {
				path: args.path,
				trace_id: args.trace_id,
				query: args.query,
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				token_id: args.token_id,
				read_profile: args.read_profile,
				expansion_mode: args.expansion_mode,
				expanded_queries: args.expanded_queries,
				allowed_scopes: args.allowed_scopes,
				candidate_count,
				filtered_candidate_count,
				snippet_count,
				scored_count,
				fused_count: fused_results.len(),
				selected_count,
				top_k: args.top_k,
				query_tokens: query_tokens.as_slice(),
				structured_matches: &args.structured_matches,
				policies: &policies,
				diversity_decisions: &diversity_decisions,
				recall_candidates: filtered_candidates,
				fused_results,
				selected_results,
				relation_contexts,
				trace_candidates,
				recursive_retrieval: args.recursive_retrieval.as_ref(),
				now,
				ranking_override: &args.ranking_override,
				filter_impact,
				payload_level: args.payload_level,
			})
			.await?;

		Ok(SearchResponse {
			trace_id: args.trace_id,
			items,
			trajectory_summary: Some(trajectory_summary),
		})
	}
}
