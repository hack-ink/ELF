use serde::Deserialize;

/// Query-time search settings.
#[derive(Debug, Deserialize)]
pub struct Search {
	/// Query expansion behavior.
	pub expansion: SearchExpansion,
	/// Dynamic-expansion trigger thresholds.
	pub dynamic: SearchDynamic,
	/// Prefilter candidate cap.
	pub prefilter: SearchPrefilter,
	/// Search cache settings.
	pub cache: SearchCache,
	/// Explainability retention settings.
	pub explain: SearchExplain,
	/// Recursive retrieval traversal settings.
	pub recursive: SearchRecursive,
	/// Graph-context enrichment settings.
	pub graph_context: SearchGraphContext,
}

/// Query expansion settings.
#[derive(Debug, Deserialize)]
pub struct SearchExpansion {
	/// Expansion mode such as `off`, `always`, or `dynamic`.
	pub mode: String,
	/// Maximum number of expansion queries emitted.
	pub max_queries: u32,
	/// Whether the original query is retained alongside expansions.
	pub include_original: bool,
}

/// Thresholds that determine when dynamic expansion is activated.
#[derive(Debug, Deserialize)]
pub struct SearchDynamic {
	/// Minimum initial candidate count before dynamic expansion is skipped.
	pub min_candidates: u32,
	/// Minimum top score before dynamic expansion is skipped.
	pub min_top_score: f32,
}

/// Candidate prefilter settings.
#[derive(Debug, Deserialize)]
pub struct SearchPrefilter {
	/// Maximum number of candidates kept before later stages.
	pub max_candidates: u32,
}

/// Cache settings for expansion and rerank outputs.
#[derive(Debug, Deserialize)]
pub struct SearchCache {
	/// Whether search caching is enabled.
	pub enabled: bool,
	/// TTL in days for cached expansion outputs.
	pub expansion_ttl_days: i64,
	/// TTL in days for cached rerank outputs.
	pub rerank_ttl_days: i64,
	/// Optional upper bound on cached payload size in bytes.
	pub max_payload_bytes: Option<u64>,
}

/// Search explainability retention and write-path settings.
#[derive(Debug, Deserialize)]
pub struct SearchExplain {
	/// Retention window for explain rows in days.
	pub retention_days: i64,
	/// Whether candidate snapshots are captured.
	pub capture_candidates: bool,
	/// Retention window for candidate snapshots in days.
	pub candidate_retention_days: i64,
	/// Explainability write mode.
	pub write_mode: String,
}

/// Recursive retrieval traversal limits.
#[derive(Debug, Deserialize)]
pub struct SearchRecursive {
	/// Whether recursive retrieval is enabled.
	pub enabled: bool,
	/// Maximum recursion depth.
	pub max_depth: u32,
	/// Maximum children expanded per node.
	pub max_children_per_node: u32,
	/// Maximum nodes retained per scope.
	pub max_nodes_per_scope: u32,
	/// Maximum nodes retained across the whole traversal.
	pub max_total_nodes: u32,
}

/// Graph-context enrichment limits applied to search responses.
#[derive(Debug, Deserialize)]
pub struct SearchGraphContext {
	/// Whether graph-context enrichment is enabled.
	pub enabled: bool,
	/// Maximum facts attached to one response item.
	pub max_facts_per_item: u32,
	/// Maximum evidence notes attached to one fact.
	pub max_evidence_notes_per_fact: u32,
}
