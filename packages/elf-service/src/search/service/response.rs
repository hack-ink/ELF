use crate::search::{
	BuildQueryPlanArgs, DynamicGateSummary, ElfService, RawSearchExecutionContext, RawSearchPath,
	SearchRawPlannedResponse, SearchResponse,
};

impl ElfService {
	pub(in crate::search) fn build_raw_planned_response(
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
