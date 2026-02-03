use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
	pub service: Service,
	pub storage: Storage,
	pub providers: Providers,
	pub scopes: Scopes,
	pub memory: Memory,
	pub search: Search,
	pub ranking: Ranking,
	pub lifecycle: Lifecycle,
	pub security: Security,
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
pub struct Search {
	pub expansion: SearchExpansion,
	pub dynamic: SearchDynamic,
	pub prefilter: SearchPrefilter,
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
pub struct Ranking {
	pub recency_tau_days: f32,
	pub tie_breaker_weight: f32,
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
}
