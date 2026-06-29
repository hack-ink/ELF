use super::*;

/// Planned-search variant of the raw search response.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchRawPlannedResponse {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Ranked search items.
	pub items: Vec<SearchItem>,
	/// Optional condensed explain output.
	pub trajectory_summary: Option<SearchTrajectorySummary>,
	/// Query plan used for the search.
	pub query_plan: QueryPlan,
}

/// Query plan emitted by planned search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlan {
	/// Query-plan schema identifier.
	pub schema: String,
	/// Query-plan version string.
	pub version: String,
	/// Ordered planning stages.
	pub stages: Vec<QueryPlanStage>,
	/// Request intent snapshot.
	pub intent: QueryPlanIntent,
	/// Query rewrite output.
	pub rewrite: QueryPlanRewrite,
	/// Retrieval-stage plan.
	pub retrieval_stages: Vec<QueryPlanRetrievalStage>,
	/// Fusion-policy snapshot.
	pub fusion_policy: QueryPlanFusionPolicy,
	/// Rerank-policy snapshot.
	pub rerank_policy: QueryPlanRerankPolicy,
	/// Budget snapshot.
	pub budget: QueryPlanBudget,
}

/// One stage in a query plan.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanStage {
	/// Stage name.
	pub name: String,
	/// Free-form stage details.
	pub details: Value,
}

/// Request intent captured in a query plan.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanIntent {
	/// Original search query text.
	pub query: String,
	/// Tenant to search within.
	pub tenant_id: String,
	/// Project to search within.
	pub project_id: String,
	/// Agent requesting the search.
	pub agent_id: String,
	/// Read profile used for the search.
	pub read_profile: String,
	/// Scopes allowed by the read profile.
	pub allowed_scopes: Vec<String>,
}

/// Rewrite section of a query plan.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanRewrite {
	/// Expansion mode label.
	pub expansion_mode: String,
	/// Expanded query strings.
	pub expanded_queries: Vec<String>,
	/// Dynamic-gate summary.
	pub dynamic_gate: QueryPlanDynamicGate,
}

/// Dynamic-query-expansion gate summary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanDynamicGate {
	/// Whether the dynamic gate was considered.
	pub considered: bool,
	/// Whether the dynamic gate decided to expand.
	pub should_expand: Option<bool>,
	/// Candidate count observed by the gate.
	pub observed_candidates: Option<u32>,
	/// Top score observed by the gate.
	pub observed_top_score: Option<f32>,
	/// Minimum candidates threshold.
	pub min_candidates: u32,
	/// Minimum top-score threshold.
	pub min_top_score: f32,
}

/// Retrieval-stage entry in a query plan.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanRetrievalStage {
	/// Stage name.
	pub name: String,
	/// Retrieval source label.
	pub source: String,
	/// Whether the stage is enabled.
	pub enabled: bool,
	/// Candidate limit for the stage.
	pub candidate_limit: u32,
}

/// Fusion-policy snapshot used during search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanFusionPolicy {
	/// Fusion strategy label.
	pub strategy: String,
	/// Weight for fusion retrieval.
	pub fusion_weight: f32,
	/// Weight for structured-field retrieval.
	pub structured_field_weight: f32,
	/// Weight for recursive retrieval.
	pub recursive_weight: f32,
	/// Priority for fusion retrieval.
	pub fusion_priority: u32,
	/// Priority for structured-field retrieval.
	pub structured_field_priority: u32,
	/// Priority for recursive retrieval.
	pub recursive_priority: u32,
}

/// One blend segment in the rerank policy.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanBlendSegment {
	/// Highest retrieval rank covered by the segment.
	pub max_retrieval_rank: u32,
	/// Retrieval weight applied within the segment.
	pub retrieval_weight: f32,
}

/// Rerank-policy snapshot used during search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanRerankPolicy {
	/// Provider identifier.
	pub provider_id: String,
	/// Model identifier.
	pub model: String,
	/// Whether blend ranking was enabled.
	pub blend_enabled: bool,
	/// Rerank normalization label.
	pub rerank_normalization: String,
	/// Retrieval normalization label.
	pub retrieval_normalization: String,
	/// Blend segments used by the policy.
	pub blend_segments: Vec<QueryPlanBlendSegment>,
	/// Whether diversity ranking was enabled.
	pub diversity_enabled: bool,
	/// Diversity similarity threshold.
	pub diversity_sim_threshold: f32,
	/// Diversity MMR lambda.
	pub diversity_mmr_lambda: f32,
	/// Diversity max-skips limit.
	pub diversity_max_skips: u32,
}

/// Budget snapshot used during search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueryPlanBudget {
	/// Final top-k budget.
	pub top_k: u32,
	/// Candidate-k budget.
	pub candidate_k: u32,
	/// Prefilter candidate cap.
	pub prefilter_max_candidates: u32,
	/// Query-expansion cap.
	pub expansion_max_queries: u32,
	/// Whether ranking caches were enabled.
	pub cache_enabled: bool,
}
