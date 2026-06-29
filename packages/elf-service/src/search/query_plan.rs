use super::*;

const QUERY_PLAN_SCHEMA: &str = "elf.search.query_plan";
const QUERY_PLAN_VERSION: &str = "v1";

impl ElfService {
	pub(super) fn build_query_plan(&self, args: BuildQueryPlanArgs<'_>) -> QueryPlan {
		let allowed_scopes = sorted_unique_strings(args.allowed_scopes.to_vec());
		let expanded_queries = sorted_unique_strings(args.expanded_queries);
		let retrieval_stages = self.build_query_plan_retrieval_stages(
			args.candidate_k,
			args.retrieval_sources_policy,
			args.recursive_enabled,
		);
		let rewrite =
			self.build_query_plan_rewrite(args.expansion_mode, expanded_queries, args.dynamic_gate);
		let fusion_policy = self.build_query_plan_fusion_policy(args.retrieval_sources_policy);
		let rerank_policy = self.build_query_plan_rerank_policy(args.policies);
		let budget = self.build_query_plan_budget(args.top_k, args.candidate_k);
		let stages = Self::build_query_plan_stages(QueryPlanStagesArgs {
			path: args.path,
			query: args.query,
			read_profile: args.read_profile,
			allowed_scope_count: allowed_scopes.len(),
			rewrite: &rewrite,
			retrieval_stages: &retrieval_stages,
			fusion_policy: &fusion_policy,
			rerank_policy: &rerank_policy,
			budget: &budget,
		});

		QueryPlan {
			schema: QUERY_PLAN_SCHEMA.to_string(),
			version: QUERY_PLAN_VERSION.to_string(),
			stages,
			intent: QueryPlanIntent {
				query: args.query.to_string(),
				tenant_id: args.tenant_id.to_string(),
				project_id: args.project_id.to_string(),
				agent_id: args.agent_id.to_string(),
				read_profile: args.read_profile.to_string(),
				allowed_scopes,
			},
			rewrite,
			retrieval_stages,
			fusion_policy,
			rerank_policy,
			budget,
		}
	}

	fn build_query_plan_retrieval_stages(
		&self,
		candidate_k: u32,
		retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
		recursive_enabled: bool,
	) -> Vec<QueryPlanRetrievalStage> {
		let mut stages = vec![
			QueryPlanRetrievalStage {
				name: "fusion_dense_bm25".to_string(),
				source: "qdrant_fusion".to_string(),
				enabled: true,
				candidate_limit: candidate_k,
			},
			QueryPlanRetrievalStage {
				name: "structured_field_vector".to_string(),
				source: "postgres_vector".to_string(),
				enabled: retrieval_sources_policy.structured_field_weight > 0.0,
				candidate_limit: candidate_k,
			},
		];

		if recursive_enabled {
			stages.push(QueryPlanRetrievalStage {
				name: "recursive_scope".to_string(),
				source: "scope_graph".to_string(),
				enabled: retrieval_sources_policy.recursive_weight > 0.0,
				candidate_limit: candidate_k,
			});
		}

		stages
	}

	fn build_query_plan_rewrite(
		&self,
		expansion_mode: ExpansionMode,
		expanded_queries: Vec<String>,
		dynamic_gate: DynamicGateSummary,
	) -> QueryPlanRewrite {
		QueryPlanRewrite {
			expansion_mode: ranking::expansion_mode_label(expansion_mode).to_string(),
			expanded_queries,
			dynamic_gate: QueryPlanDynamicGate {
				considered: dynamic_gate.considered,
				should_expand: dynamic_gate.should_expand,
				observed_candidates: dynamic_gate.observed_candidates,
				observed_top_score: dynamic_gate.observed_top_score,
				min_candidates: self.cfg.search.dynamic.min_candidates,
				min_top_score: self.cfg.search.dynamic.min_top_score,
			},
		}
	}

	fn build_query_plan_fusion_policy(
		&self,
		retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
	) -> QueryPlanFusionPolicy {
		QueryPlanFusionPolicy {
			strategy: "weighted_merge".to_string(),
			fusion_weight: retrieval_sources_policy.fusion_weight,
			structured_field_weight: retrieval_sources_policy.structured_field_weight,
			recursive_weight: retrieval_sources_policy.recursive_weight,
			fusion_priority: retrieval_sources_policy.fusion_priority,
			structured_field_priority: retrieval_sources_policy.structured_field_priority,
			recursive_priority: retrieval_sources_policy.recursive_priority,
		}
	}

	fn build_query_plan_rerank_policy(
		&self,
		policies: &FinishSearchPolicies,
	) -> QueryPlanRerankPolicy {
		QueryPlanRerankPolicy {
			provider_id: self.cfg.providers.rerank.provider_id.clone(),
			model: self.cfg.providers.rerank.model.clone(),
			blend_enabled: policies.blend_policy.enabled,
			rerank_normalization: policies.blend_policy.rerank_normalization.as_str().to_string(),
			retrieval_normalization: policies
				.blend_policy
				.retrieval_normalization
				.as_str()
				.to_string(),
			blend_segments: policies
				.blend_policy
				.segments
				.iter()
				.map(|segment| QueryPlanBlendSegment {
					max_retrieval_rank: segment.max_retrieval_rank,
					retrieval_weight: segment.retrieval_weight,
				})
				.collect(),
			diversity_enabled: policies.diversity_policy.enabled,
			diversity_sim_threshold: policies.diversity_policy.sim_threshold,
			diversity_mmr_lambda: policies.diversity_policy.mmr_lambda,
			diversity_max_skips: policies.diversity_policy.max_skips,
		}
	}

	fn build_query_plan_budget(&self, top_k: u32, candidate_k: u32) -> QueryPlanBudget {
		QueryPlanBudget {
			top_k,
			candidate_k,
			prefilter_max_candidates: self.cfg.search.prefilter.max_candidates,
			expansion_max_queries: self.cfg.search.expansion.max_queries,
			cache_enabled: self.cfg.search.cache.enabled,
		}
	}

	fn build_query_plan_stages(args: QueryPlanStagesArgs<'_>) -> Vec<QueryPlanStage> {
		vec![
			QueryPlanStage {
				name: "intent".to_string(),
				details: serde_json::json!({
					"path": raw_search_path_label(args.path),
					"query": args.query,
					"read_profile": args.read_profile,
					"allowed_scope_count": args.allowed_scope_count,
				}),
			},
			QueryPlanStage {
				name: "rewrite".to_string(),
				details: serde_json::json!({
					"expansion_mode": args.rewrite.expansion_mode.as_str(),
					"expanded_query_count": args.rewrite.expanded_queries.len(),
					"dynamic_gate_considered": args.rewrite.dynamic_gate.considered,
					"dynamic_gate_should_expand": args.rewrite.dynamic_gate.should_expand,
				}),
			},
			QueryPlanStage {
				name: "retrieval".to_string(),
				details: serde_json::json!({
					"stages": args.retrieval_stages,
				}),
			},
			QueryPlanStage {
				name: "fusion".to_string(),
				details: serde_json::json!({
					"strategy": args.fusion_policy.strategy.as_str(),
					"fusion_weight": args.fusion_policy.fusion_weight,
					"structured_field_weight": args.fusion_policy.structured_field_weight,
				}),
			},
			QueryPlanStage {
				name: "rerank".to_string(),
				details: serde_json::json!({
					"provider_id": args.rerank_policy.provider_id.as_str(),
					"model": args.rerank_policy.model.as_str(),
					"blend_enabled": args.rerank_policy.blend_enabled,
					"diversity_enabled": args.rerank_policy.diversity_enabled,
				}),
			},
			QueryPlanStage {
				name: "budget".to_string(),
				details: serde_json::json!({
					"top_k": args.budget.top_k,
					"candidate_k": args.budget.candidate_k,
					"prefilter_max_candidates": args.budget.prefilter_max_candidates,
					"expansion_max_queries": args.budget.expansion_max_queries,
					"cache_enabled": args.budget.cache_enabled,
				}),
			},
		]
	}
}
