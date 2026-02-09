use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
	pub service: Service,
	pub storage: Storage,
	pub providers: Providers,
	pub scopes: Scopes,
	pub memory: Memory,
	pub chunking: Chunking,
	pub search: Search,
	pub ranking: Ranking,
	pub lifecycle: Lifecycle,
	pub security: Security,
	pub context: Option<Context>,
	pub mcp: Option<McpContext>,
}

#[derive(Debug, Deserialize)]
pub struct Context {
	/// Optional. Map keys are either "<tenant_id>:<project_id>" or "<project_id>".
	pub project_descriptions: Option<HashMap<String, String>>,
	/// Optional. Map keys are scope labels, e.g. "project_shared".
	pub scope_descriptions: Option<HashMap<String, String>>,
	/// Optional. Additive boost applied to final scores when a query's tokens match a scope
	/// description.
	pub scope_boost_weight: Option<f32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct McpContext {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	#[serde(default = "default_read_profile")]
	pub read_profile: String,
}

#[derive(Debug, Deserialize)]
pub struct Service {
	pub http_bind: String,
	pub mcp_bind: String,
	pub admin_bind: String,
	pub log_level: String,
}

#[derive(Debug, Deserialize)]
pub struct Storage {
	pub postgres: Postgres,
	pub qdrant: Qdrant,
}

#[derive(Debug, Deserialize)]
pub struct Postgres {
	pub dsn: String,
	pub pool_max_conns: u32,
}

#[derive(Debug, Deserialize)]
pub struct Qdrant {
	pub url: String,
	pub collection: String,
	pub vector_dim: u32,
}

#[derive(Debug, Deserialize)]
pub struct Providers {
	pub embedding: EmbeddingProviderConfig,
	pub rerank: ProviderConfig,
	pub llm_extractor: LlmProviderConfig,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingProviderConfig {
	pub provider_id: String,
	pub api_base: String,
	pub api_key: String,
	pub path: String,
	pub model: String,
	pub dimensions: u32,
	pub timeout_ms: u64,
	pub default_headers: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
	pub provider_id: String,
	pub api_base: String,
	pub api_key: String,
	pub path: String,
	pub model: String,
	pub timeout_ms: u64,
	pub default_headers: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct LlmProviderConfig {
	pub provider_id: String,
	pub api_base: String,
	pub api_key: String,
	pub path: String,
	pub model: String,
	pub temperature: f32,
	pub timeout_ms: u64,
	pub default_headers: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Scopes {
	pub allowed: Vec<String>,
	pub read_profiles: ReadProfiles,
	pub precedence: ScopePrecedence,
	pub write_allowed: ScopeWriteAllowed,
}

#[derive(Debug, Deserialize)]
pub struct ReadProfiles {
	pub private_only: Vec<String>,
	pub private_plus_project: Vec<String>,
	pub all_scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScopePrecedence {
	pub agent_private: i32,
	pub project_shared: i32,
	pub org_shared: i32,
}

#[derive(Debug, Deserialize)]
pub struct ScopeWriteAllowed {
	pub agent_private: bool,
	pub project_shared: bool,
	pub org_shared: bool,
}

#[derive(Debug, Deserialize)]
pub struct Memory {
	pub max_notes_per_add_event: u32,
	pub max_note_chars: u32,
	pub dup_sim_threshold: f32,
	pub update_sim_threshold: f32,
	pub candidate_k: u32,
	pub top_k: u32,
}

#[derive(Debug, Deserialize)]
pub struct Chunking {
	pub enabled: bool,
	pub max_tokens: u32,
	pub overlap_tokens: u32,
	pub tokenizer_repo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Search {
	pub expansion: SearchExpansion,
	pub dynamic: SearchDynamic,
	pub prefilter: SearchPrefilter,
	pub cache: SearchCache,
	pub explain: SearchExplain,
}

#[derive(Debug, Deserialize)]
pub struct SearchExpansion {
	pub mode: String,
	pub max_queries: u32,
	pub include_original: bool,
}

#[derive(Debug, Deserialize)]
pub struct SearchDynamic {
	pub min_candidates: u32,
	pub min_top_score: f32,
}

#[derive(Debug, Deserialize)]
pub struct SearchPrefilter {
	pub max_candidates: u32,
}

#[derive(Debug, Deserialize)]
pub struct SearchCache {
	pub enabled: bool,
	pub expansion_ttl_days: i64,
	pub rerank_ttl_days: i64,
	pub max_payload_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchExplain {
	pub retention_days: i64,
	#[serde(default)]
	pub capture_candidates: bool,
	#[serde(default = "default_candidate_retention_days")]
	pub candidate_retention_days: i64,
	#[serde(default = "default_explain_write_mode")]
	pub write_mode: String,
}

fn default_candidate_retention_days() -> i64 {
	2
}

fn default_explain_write_mode() -> String {
	"outbox".to_string()
}

#[derive(Debug, Deserialize)]
pub struct Ranking {
	pub recency_tau_days: f32,
	pub tie_breaker_weight: f32,
	#[serde(default)]
	pub blend: RankingBlend,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RankingBlend {
	pub enabled: bool,
	pub rerank_normalization: String,
	pub retrieval_normalization: String,
	pub segments: Vec<RankingBlendSegment>,
}
impl Default for RankingBlend {
	fn default() -> Self {
		Self {
			enabled: true,
			rerank_normalization: "rank".to_string(),
			retrieval_normalization: "rank".to_string(),
			segments: vec![
				RankingBlendSegment { max_retrieval_rank: 3, retrieval_weight: 0.8 },
				RankingBlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.5 },
				RankingBlendSegment { max_retrieval_rank: 1_000_000, retrieval_weight: 0.2 },
			],
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct RankingBlendSegment {
	pub max_retrieval_rank: u32,
	pub retrieval_weight: f32,
}

#[derive(Debug, Deserialize)]
pub struct Lifecycle {
	pub ttl_days: TtlDays,
	pub purge_deleted_after_days: i64,
	pub purge_deprecated_after_days: i64,
}

#[derive(Debug, Deserialize)]
pub struct TtlDays {
	pub plan: i64,
	pub fact: i64,
	pub preference: i64,
	pub constraint: i64,
	pub decision: i64,
	pub profile: i64,
}

#[derive(Debug, Deserialize)]
pub struct Security {
	pub bind_localhost_only: bool,
	pub reject_cjk: bool,
	pub redact_secrets_on_write: bool,
	pub evidence_min_quotes: u32,
	pub evidence_max_quotes: u32,
	pub evidence_max_quote_chars: u32,
	#[serde(default)]
	pub api_auth_token: Option<String>,
	#[serde(default)]
	pub admin_auth_token: Option<String>,
}

fn default_read_profile() -> String {
	"private_plus_project".to_string()
}
