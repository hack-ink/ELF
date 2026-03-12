use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value};

/// Complete ELF runtime configuration loaded from `elf.toml`.
#[derive(Debug, Deserialize)]
pub struct Config {
	/// Network bind and log-level settings for ELF services.
	pub service: Service,
	/// Postgres and Qdrant storage backends.
	pub storage: Storage,
	/// Provider settings for embedding, rerank, and extraction calls.
	pub providers: Providers,
	/// Scope labels, read profiles, precedence, and write permissions.
	pub scopes: Scopes,
	/// Write-path limits and memory policy controls.
	pub memory: Memory,
	/// Sentence-aware chunking settings used by ingestion paths.
	pub chunking: Chunking,
	/// Query expansion, caching, explainability, and recursive search settings.
	pub search: Search,
	/// Retrieval ranking, blending, and diversity settings.
	pub ranking: Ranking,
	/// TTL and purge windows for stored notes.
	pub lifecycle: Lifecycle,
	/// Bind-localhost, evidence, and auth settings.
	pub security: Security,
	/// Optional retrieval context metadata used to boost project and scope matches.
	pub context: Option<Context>,
	/// Optional MCP forwarding context used by `elf-mcp`.
	pub mcp: Option<McpContext>,
}

/// Optional metadata used to improve retrieval disambiguation across projects and scopes.
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

/// Static forwarding context attached by `elf-mcp` to proxied requests.
#[derive(Clone, Debug, Deserialize)]
pub struct McpContext {
	/// Tenant identifier attached to proxied MCP requests.
	pub tenant_id: String,
	/// Project identifier attached to proxied MCP requests.
	pub project_id: String,
	/// Agent identifier attached to proxied MCP requests.
	pub agent_id: String,
	/// Read profile attached to proxied MCP requests.
	pub read_profile: String,
}

/// Bind addresses and logging settings for ELF services.
#[derive(Debug, Deserialize)]
pub struct Service {
	/// Bind address for the public HTTP API.
	pub http_bind: String,
	/// Bind address for the MCP server entrypoint.
	pub mcp_bind: String,
	/// Bind address for the admin HTTP API.
	pub admin_bind: String,
	/// Default service log level.
	pub log_level: String,
}

/// Storage backend configuration for persisted note and document data.
#[derive(Debug, Deserialize)]
pub struct Storage {
	/// Postgres source-of-truth settings.
	pub postgres: Postgres,
	/// Qdrant derived-index settings.
	pub qdrant: Qdrant,
}

/// Postgres connection settings.
#[derive(Debug, Deserialize)]
pub struct Postgres {
	/// Postgres DSN used by ELF services.
	pub dsn: String,
	/// Maximum number of pooled Postgres connections.
	pub pool_max_conns: u32,
}

/// Qdrant collection settings for note and document vectors.
#[derive(Debug, Deserialize)]
pub struct Qdrant {
	/// Qdrant base URL used by clients in this workspace.
	pub url: String,
	/// Primary notes collection name.
	pub collection: String,
	/// Document-chunk collection name.
	#[serde(default = "default_docs_collection")]
	pub docs_collection: String,
	/// Vector dimension expected by both note and document collections.
	pub vector_dim: u32,
}

/// Provider configuration bundle for all external model calls.
#[derive(Debug, Deserialize)]
pub struct Providers {
	/// Embedding provider used for vector generation.
	pub embedding: EmbeddingProviderConfig,
	/// Rerank provider used for late-stage scoring.
	pub rerank: ProviderConfig,
	/// LLM provider used by extraction flows such as `add_event`.
	pub llm_extractor: LlmProviderConfig,
}

/// Embedding-provider settings.
#[derive(Debug, Deserialize)]
pub struct EmbeddingProviderConfig {
	/// Provider implementation identifier.
	pub provider_id: String,
	/// Base URL for embedding API requests.
	pub api_base: String,
	/// Non-empty API key for embedding requests.
	pub api_key: String,
	/// Request path appended to `api_base`.
	pub path: String,
	/// Embedding model identifier.
	pub model: String,
	/// Expected embedding vector dimension.
	pub dimensions: u32,
	/// Request timeout in milliseconds.
	pub timeout_ms: u64,
	/// Extra HTTP headers sent with embedding requests.
	pub default_headers: Map<String, Value>,
}

/// Generic provider settings shared by non-embedding APIs such as rerank.
#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
	/// Provider implementation identifier.
	pub provider_id: String,
	/// Base URL for provider API requests.
	pub api_base: String,
	/// Non-empty API key for provider requests.
	pub api_key: String,
	/// Request path appended to `api_base`.
	pub path: String,
	/// Provider model identifier.
	pub model: String,
	/// Request timeout in milliseconds.
	pub timeout_ms: u64,
	/// Extra HTTP headers sent with provider requests.
	pub default_headers: Map<String, Value>,
}

/// LLM extractor provider settings.
#[derive(Debug, Deserialize)]
pub struct LlmProviderConfig {
	/// Provider implementation identifier.
	pub provider_id: String,
	/// Base URL for extraction API requests.
	pub api_base: String,
	/// Non-empty API key for extraction requests.
	pub api_key: String,
	/// Request path appended to `api_base`.
	pub path: String,
	/// LLM model identifier.
	pub model: String,
	/// Sampling temperature for extraction requests.
	pub temperature: f32,
	/// Request timeout in milliseconds.
	pub timeout_ms: u64,
	/// Extra HTTP headers sent with extraction requests.
	pub default_headers: Map<String, Value>,
}

/// Scope labels and access policy used by memory operations.
#[derive(Debug, Deserialize)]
pub struct Scopes {
	/// All scope labels allowed by this deployment.
	pub allowed: Vec<String>,
	/// Scope sets referenced by named read profiles.
	pub read_profiles: ReadProfiles,
	/// Relative precedence used when multiple scopes are eligible.
	pub precedence: ScopePrecedence,
	/// Scope-level write permissions.
	pub write_allowed: ScopeWriteAllowed,
}

/// Scope lists used by named read profiles.
#[derive(Debug, Deserialize)]
pub struct ReadProfiles {
	/// Scope set for `private_only`.
	pub private_only: Vec<String>,
	/// Scope set for `private_plus_project`.
	pub private_plus_project: Vec<String>,
	/// Scope set for `all_scopes`.
	pub all_scopes: Vec<String>,
}

/// Integer precedence used to break ties between scope classes.
#[derive(Debug, Deserialize)]
pub struct ScopePrecedence {
	/// Precedence assigned to `agent_private`.
	pub agent_private: i32,
	/// Precedence assigned to `project_shared`.
	pub project_shared: i32,
	/// Precedence assigned to `org_shared`.
	pub org_shared: i32,
}

/// Scope-level write toggles.
#[derive(Debug, Deserialize)]
pub struct ScopeWriteAllowed {
	/// Whether writes to `agent_private` are allowed.
	pub agent_private: bool,
	/// Whether writes to `project_shared` are allowed.
	pub project_shared: bool,
	/// Whether writes to `org_shared` are allowed.
	pub org_shared: bool,
}

/// Write-path limits and policy controls for note ingestion.
#[derive(Debug, Deserialize)]
pub struct Memory {
	/// Maximum number of notes accepted per `add_event` request.
	pub max_notes_per_add_event: u32,
	/// Maximum character length for an individual note.
	pub max_note_chars: u32,
	/// Similarity threshold for duplicate detection.
	pub dup_sim_threshold: f32,
	/// Similarity threshold for update-vs-insert decisions.
	pub update_sim_threshold: f32,
	/// Candidate pool size used before final top-k selection.
	pub candidate_k: u32,
	/// Final top-k size for note retrieval.
	pub top_k: u32,
	/// Optional downgrade rules applied after base memory decisions.
	#[serde(default)]
	pub policy: MemoryPolicy,
}

/// Collection of memory-policy downgrade rules.
#[derive(Debug, Default, Deserialize)]
pub struct MemoryPolicy {
	/// Ordered policy rules evaluated against note type, scope, and scores.
	pub rules: Vec<MemoryPolicyRule>,
}

/// A single memory-policy rule matched by note metadata and confidence/importance thresholds.
#[derive(Debug, Default, Deserialize)]
pub struct MemoryPolicyRule {
	/// Optional note type selector.
	pub note_type: Option<String>,
	/// Optional scope selector.
	pub scope: Option<String>,
	/// Optional minimum confidence required for the rule to match.
	pub min_confidence: Option<f32>,
	/// Optional minimum importance required for the rule to match.
	pub min_importance: Option<f32>,
}

/// Sentence-aware token chunking settings.
#[derive(Debug, Deserialize)]
pub struct Chunking {
	/// Whether chunking support is enabled.
	pub enabled: bool,
	/// Maximum tokens allowed in one chunk.
	pub max_tokens: u32,
	/// Number of tail tokens overlapped into the next chunk.
	pub overlap_tokens: u32,
	/// Hugging Face tokenizer repo used for token counting.
	pub tokenizer_repo: String,
}

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
	#[serde(default)]
	pub recursive: SearchRecursive,
	/// Graph-context enrichment settings.
	#[serde(default)]
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
#[serde(default)]
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
impl Default for SearchRecursive {
	fn default() -> Self {
		Self {
			enabled: false,
			max_depth: 2,
			max_children_per_node: 4,
			max_nodes_per_scope: 32,
			max_total_nodes: 256,
		}
	}
}

/// Graph-context enrichment limits applied to search responses.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SearchGraphContext {
	/// Whether graph-context enrichment is enabled.
	pub enabled: bool,
	/// Maximum facts attached to one response item.
	pub max_facts_per_item: u32,
	/// Maximum evidence notes attached to one fact.
	pub max_evidence_notes_per_fact: u32,
}
impl Default for SearchGraphContext {
	fn default() -> Self {
		Self { enabled: false, max_facts_per_item: 16, max_evidence_notes_per_fact: 16 }
	}
}

/// Ranking settings for retrieval and rerank fusion.
#[derive(Debug, Deserialize)]
pub struct Ranking {
	/// Recency decay window in days.
	pub recency_tau_days: f32,
	/// Small deterministic tie-breaker weight.
	pub tie_breaker_weight: f32,
	/// Retrieval/rerank blending configuration.
	pub blend: RankingBlend,
	/// Optional deterministic scoring overlays.
	pub deterministic: RankingDeterministic,
	/// Diversity settings applied during selection.
	pub diversity: RankingDiversity,
	/// Source weighting and priority between fusion and structured fields.
	pub retrieval_sources: RankingRetrievalSources,
}

/// Deterministic ranking overlays applied on top of model scores.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministic {
	/// Whether deterministic overlays are enabled.
	pub enabled: bool,
	/// Lexical-overlap term settings.
	pub lexical: RankingDeterministicLexical,
	/// Historical-hit term settings.
	pub hits: RankingDeterministicHits,
	/// Decay term settings.
	pub decay: RankingDeterministicDecay,
}

/// Lexical-overlap deterministic term.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministicLexical {
	/// Whether the lexical term is enabled.
	pub enabled: bool,
	/// Weight assigned to the lexical term.
	pub weight: f32,
	/// Minimum overlap ratio required before the term applies.
	pub min_ratio: f32,
	/// Maximum number of query terms examined.
	pub max_query_terms: u32,
	/// Maximum number of text terms examined.
	pub max_text_terms: u32,
}

/// Historical-hit deterministic term.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministicHits {
	/// Whether the hits term is enabled.
	pub enabled: bool,
	/// Weight assigned to the hits term.
	pub weight: f32,
	/// Half-saturation parameter for hit-count scaling.
	pub half_saturation: f32,
	/// Decay window in days for the last-hit component.
	pub last_hit_tau_days: f32,
}

/// Decay-based deterministic term.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministicDecay {
	/// Whether the decay term is enabled.
	pub enabled: bool,
	/// Weight assigned to the decay term.
	pub weight: f32,
	/// Decay window in days.
	pub tau_days: f32,
}

/// Retrieval/rerank blending configuration.
#[derive(Debug, Deserialize)]
pub struct RankingBlend {
	/// Whether blend mode is enabled.
	pub enabled: bool,
	/// Normalization strategy applied to rerank scores.
	pub rerank_normalization: String,
	/// Normalization strategy applied to retrieval scores.
	pub retrieval_normalization: String,
	/// Retrieval-rank segments that assign retrieval weights.
	pub segments: Vec<RankingBlendSegment>,
}

/// One retrieval-rank segment used by blend mode.
#[derive(Debug, Deserialize)]
pub struct RankingBlendSegment {
	/// Inclusive maximum retrieval rank for this segment.
	pub max_retrieval_rank: u32,
	/// Retrieval weight applied within this segment.
	pub retrieval_weight: f32,
}

/// Diversity controls used when selecting final results.
#[derive(Debug, Deserialize)]
pub struct RankingDiversity {
	/// Whether diversity filtering is enabled.
	pub enabled: bool,
	/// Similarity threshold above which candidates may be skipped.
	pub sim_threshold: f32,
	/// Lambda used by MMR-style balancing.
	pub mmr_lambda: f32,
	/// Maximum number of skipped candidates before backfilling.
	pub max_skips: u32,
}

/// Source weighting and priority between fusion and structured-field retrieval.
#[derive(Debug, Deserialize)]
pub struct RankingRetrievalSources {
	/// Weight applied to fused retrieval results.
	pub fusion_weight: f32,
	/// Weight applied to structured-field matches.
	pub structured_field_weight: f32,
	/// Priority assigned to fused retrieval results.
	pub fusion_priority: u32,
	/// Priority assigned to structured-field matches.
	pub structured_field_priority: u32,
}

/// Lifecycle retention and purge settings.
#[derive(Debug, Deserialize)]
pub struct Lifecycle {
	/// Note-type-specific TTL settings.
	pub ttl_days: TtlDays,
	/// Days to retain deleted notes before purge.
	pub purge_deleted_after_days: i64,
	/// Days to retain deprecated notes before purge.
	pub purge_deprecated_after_days: i64,
}

/// TTL values in days for each note type.
#[derive(Debug, Deserialize)]
pub struct TtlDays {
	/// TTL for `plan` notes.
	pub plan: i64,
	/// TTL for `fact` notes.
	pub fact: i64,
	/// TTL for `preference` notes.
	pub preference: i64,
	/// TTL for `constraint` notes.
	pub constraint: i64,
	/// TTL for `decision` notes.
	pub decision: i64,
	/// TTL for `profile` notes.
	pub profile: i64,
}

/// Request security, evidence, and auth settings.
#[derive(Debug, Deserialize)]
pub struct Security {
	/// Whether services must bind only to loopback interfaces.
	pub bind_localhost_only: bool,
	/// Whether non-English input is rejected at the API boundary.
	pub reject_non_english: bool,
	/// Whether secret-like text is redacted before write.
	pub redact_secrets_on_write: bool,
	/// Minimum number of quotes required for evidence binding.
	pub evidence_min_quotes: u32,
	/// Maximum number of quotes allowed for evidence binding.
	pub evidence_max_quotes: u32,
	/// Maximum characters allowed in one evidence quote.
	pub evidence_max_quote_chars: u32,
	/// Authentication mode such as `off` or `static_keys`.
	pub auth_mode: String,
	/// Static bearer-token entries used when `auth_mode` is `static_keys`.
	#[serde(default)]
	pub auth_keys: Vec<SecurityAuthKey>,
}

/// A single static bearer-token entry.
#[derive(Debug, Deserialize)]
pub struct SecurityAuthKey {
	/// Stable token identifier used for auditing.
	pub token_id: String,
	/// Bearer token value matched from incoming requests.
	pub token: String,
	/// Tenant identifier granted by this token.
	pub tenant_id: String,
	/// Project identifier granted by this token.
	pub project_id: String,

	/// Optional agent identifier restriction.
	pub agent_id: Option<String>,
	/// Read profile granted by this token.
	pub read_profile: String,
	/// Role assigned to this token.
	pub role: SecurityAuthRole,
}

/// Role values accepted by static auth keys.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityAuthRole {
	/// Standard user token.
	User,
	/// Admin token with elevated write privileges.
	Admin,
	/// Super-admin token for global admin operations.
	SuperAdmin,
}

fn default_docs_collection() -> String {
	"doc_chunks_v1".to_string()
}
