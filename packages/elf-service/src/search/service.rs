use super::*;

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
		let filter = build_search_filter(
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

		validate_search_request_inputs(
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
			.map_err(|err| crate::Error::InvalidRequest { message: err.to_string() })?;
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

	pub(super) async fn finish_search(&self, args: FinishSearchArgs<'_>) -> Result<SearchResponse> {
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
