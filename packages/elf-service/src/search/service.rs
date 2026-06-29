use crate::{
	Error,
	search::{
		self, BuildQueryPlanArgs, DynamicGateSummary, ElfService, ExpansionMode, FinishSearchArgs,
		HashMap, MAX_CANDIDATE_K, MaybeDynamicSearchArgs, RawSearchExecutionContext, RawSearchPath,
		Result, SearchFilter, SearchRawPlannedResponse, SearchRequest, SearchResponse,
		SearchRetrievalArgs, Uuid, ranking,
	},
};

impl ElfService {
	/// Runs the quick raw-search path and returns ranked items without a query plan.
	pub async fn search_raw_quick(&self, req: SearchRequest) -> Result<SearchResponse> {
		self.execute_search_raw_path(req, RawSearchPath::Quick).await.map(|response| {
			SearchResponse {
				trace_id: response.trace_id,
				items: response.items,
				trajectory_summary: response.trajectory_summary,
			}
		})
	}

	/// Runs the planned raw-search path and returns ranked items plus a query plan.
	pub async fn search_raw_planned(&self, req: SearchRequest) -> Result<SearchRawPlannedResponse> {
		self.execute_search_raw_path(req, RawSearchPath::Planned).await
	}

	/// Runs the default raw-search path and returns ranked items.
	pub async fn search_raw(&self, req: SearchRequest) -> Result<SearchResponse> {
		self.search_raw_planned(req).await.map(|response| SearchResponse {
			trace_id: response.trace_id,
			items: response.items,
			trajectory_summary: response.trajectory_summary,
		})
	}

	async fn execute_search_raw_path(
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

	fn prepare_raw_search_execution(
		&self,
		req: SearchRequest,
		path: RawSearchPath,
	) -> Result<RawSearchExecutionContext> {
		let tenant_id = req.tenant_id.trim().to_string();
		let project_id = req.project_id.trim().to_string();
		let agent_id = req.agent_id.trim().to_string();
		let token_id = req
			.token_id
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(|value| value.to_string());

		search::validate_search_request_inputs(
			tenant_id.as_str(),
			project_id.as_str(),
			agent_id.as_str(),
			req.query.as_str(),
		)?;

		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let candidate_k = req.candidate_k.unwrap_or(self.cfg.memory.candidate_k).max(top_k);
		let requested_candidate_k = candidate_k;
		let filter = req
			.filter
			.as_ref()
			.map(SearchFilter::parse)
			.transpose()
			.map_err(|err| Error::InvalidRequest { message: err.to_string() })?;
		let effective_candidate_k = if filter.is_some() {
			requested_candidate_k.saturating_mul(3).min(MAX_CANDIDATE_K).max(top_k)
		} else {
			requested_candidate_k
		};
		let query = req.query;
		let read_profile = req.read_profile;
		let record_hits_enabled = req.record_hits.unwrap_or(false);
		let ranking_override = req.ranking;
		let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.as_ref().and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let expansion_mode = match path {
			RawSearchPath::Quick => ExpansionMode::Off,
			RawSearchPath::Planned => ranking::resolve_expansion_mode(&self.cfg),
		};
		let trace_id = Uuid::new_v4();
		let project_context_description = self
			.resolve_project_context_description(tenant_id.as_str(), project_id.as_str())
			.map(|value| value.to_string());
		let allowed_scopes = ranking::resolve_scopes(&self.cfg, read_profile.as_str())?;
		let policies = self.resolve_finish_search_policies(ranking_override.as_ref())?;

		Ok(RawSearchExecutionContext {
			tenant_id,
			project_id,
			agent_id,
			token_id,
			top_k,
			candidate_k,
			requested_candidate_k,
			effective_candidate_k,
			filter,
			query,
			read_profile,
			payload_level: req.payload_level,
			record_hits_enabled,
			ranking_override,
			retrieval_sources_policy,
			expansion_mode,
			trace_id,
			project_context_description,
			allowed_scopes,
			policies,
		})
	}

	fn build_raw_planned_response(
		&self,
		context: &RawSearchExecutionContext,
		path: RawSearchPath,
		response: SearchResponse,
		expanded_queries: Vec<String>,
		dynamic_gate: DynamicGateSummary,
	) -> SearchRawPlannedResponse {
		let query_plan = self.build_query_plan(BuildQueryPlanArgs {
			path,
			query: context.query.as_str(),
			tenant_id: context.tenant_id.as_str(),
			project_id: context.project_id.as_str(),
			agent_id: context.agent_id.as_str(),
			read_profile: context.read_profile.as_str(),
			allowed_scopes: &context.allowed_scopes,
			expansion_mode: context.expansion_mode,
			expanded_queries,
			top_k: context.top_k,
			candidate_k: context.candidate_k,
			retrieval_sources_policy: &context.retrieval_sources_policy,
			recursive_enabled: self.cfg.search.recursive.enabled,
			policies: &context.policies,
			dynamic_gate,
		});

		SearchRawPlannedResponse {
			trace_id: response.trace_id,
			items: response.items,
			trajectory_summary: response.trajectory_summary,
			query_plan,
		}
	}
}
