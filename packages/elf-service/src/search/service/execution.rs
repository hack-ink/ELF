use crate::search::{
	self, DynamicGateSummary, ElfService, ExpansionMode, FinishSearchArgs, HashMap,
	MaybeDynamicSearchArgs, RawSearchExecutionContext, RawSearchPath, Result,
	SearchRawPlannedResponse, SearchRequest, SearchRetrievalArgs,
};

impl ElfService {
	pub(in crate::search) async fn execute_search_raw_path(
		&self,
		req: SearchRequest,
		path: RawSearchPath,
	) -> Result<SearchRawPlannedResponse> {
		let context = self.prepare_raw_search_execution(req, path)?;

		if context.allowed_scopes.is_empty() {
			return self.execute_search_raw_no_allowed_scopes(&context, path).await;
		}

		let dynamic_gate_enabled =
			path == RawSearchPath::Planned && context.expansion_mode == ExpansionMode::Dynamic;

		self.execute_search_raw_with_allowed_scopes(&context, path, dynamic_gate_enabled).await
	}

	async fn execute_search_raw_no_allowed_scopes(
		&self,
		context: &RawSearchExecutionContext,
		path: RawSearchPath,
	) -> Result<SearchRawPlannedResponse> {
		let expanded_queries = vec![context.query.clone()];
		let response = self
			.finish_search(FinishSearchArgs {
				path,
				trace_id: context.trace_id,
				query: context.query.as_str(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				token_id: context.token_id.as_deref(),
				read_profile: context.read_profile.as_str(),
				allowed_scopes: &context.allowed_scopes,
				expanded_queries: expanded_queries.clone(),
				expansion_mode: context.expansion_mode,
				candidates: Vec::new(),
				structured_matches: HashMap::new(),
				recursive_retrieval: None,
				top_k: context.top_k,
				record_hits_enabled: context.record_hits_enabled,
				ranking_override: context.ranking_override.clone(),
				payload_level: context.payload_level,
				filter: context.filter.as_ref(),
				requested_candidate_k: context.requested_candidate_k,
				effective_candidate_k: context.effective_candidate_k,
			})
			.await?;

		Ok(self.build_raw_planned_response(
			context,
			path,
			response,
			expanded_queries,
			DynamicGateSummary::default(),
		))
	}

	async fn execute_search_raw_with_allowed_scopes(
		&self,
		context: &RawSearchExecutionContext,
		path: RawSearchPath,
		dynamic_gate_enabled: bool,
	) -> Result<SearchRawPlannedResponse> {
		let filter = search::build_search_filter(
			context.tenant_id.as_str(),
			context.project_id.as_str(),
			context.agent_id.as_str(),
			&context.allowed_scopes,
		);
		let retrieval_candidate_k = if context.filter.is_some() {
			context.effective_candidate_k
		} else {
			context.candidate_k
		};
		let (baseline_vector, early_response, dynamic_gate) = self
			.maybe_finish_dynamic_search(MaybeDynamicSearchArgs {
				path,
				enabled: dynamic_gate_enabled,
				trace_id: context.trace_id,
				query: context.query.as_str(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				token_id: context.token_id.as_deref(),
				read_profile: context.read_profile.as_str(),
				allowed_scopes: &context.allowed_scopes,
				project_context_description: context.project_context_description.as_deref(),
				filter: &filter,
				service_filter: context.filter.as_ref(),
				candidate_k: retrieval_candidate_k,
				requested_candidate_k: context.requested_candidate_k,
				effective_candidate_k: context.effective_candidate_k,
				top_k: context.top_k,
				record_hits_enabled: context.record_hits_enabled,
				ranking_override: context.ranking_override.as_ref(),
				retrieval_sources_policy: &context.retrieval_sources_policy,
				payload_level: context.payload_level,
			})
			.await?;

		if let Some(response) = early_response {
			return Ok(self.build_raw_planned_response(
				context,
				path,
				response,
				vec![context.query.clone()],
				dynamic_gate,
			));
		}

		let retrieval = self
			.retrieve_search_candidates(SearchRetrievalArgs {
				query: context.query.as_str(),
				expansion_mode: context.expansion_mode,
				project_context_description: context.project_context_description.as_deref(),
				filter: &filter,
				candidate_k: retrieval_candidate_k,
				baseline_vector: baseline_vector.as_ref(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				allowed_scopes: &context.allowed_scopes,
				retrieval_sources_policy: &context.retrieval_sources_policy,
			})
			.await?;
		let expanded_queries = retrieval.expanded_queries.clone();
		let response = self
			.finish_search(FinishSearchArgs {
				path,
				trace_id: context.trace_id,
				query: context.query.as_str(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				token_id: context.token_id.as_deref(),
				read_profile: context.read_profile.as_str(),
				allowed_scopes: &context.allowed_scopes,
				expanded_queries: retrieval.expanded_queries,
				expansion_mode: context.expansion_mode,
				candidates: retrieval.candidates,
				structured_matches: retrieval.structured_matches,
				recursive_retrieval: retrieval.recursive,
				top_k: context.top_k,
				record_hits_enabled: context.record_hits_enabled,
				ranking_override: context.ranking_override.clone(),
				payload_level: context.payload_level,
				filter: context.filter.as_ref(),
				requested_candidate_k: context.requested_candidate_k,
				effective_candidate_k: context.effective_candidate_k,
			})
			.await?;

		Ok(self.build_raw_planned_response(context, path, response, expanded_queries, dynamic_gate))
	}
}
