mod chunking;
mod context;
mod lifecycle;
mod memory;
mod providers;
mod ranking;
mod scopes;
mod search;
mod security;
mod service;
mod storage;

pub use self::{
	chunking::Chunking,
	context::{Context, McpContext},
	lifecycle::{Lifecycle, TtlDays},
	memory::{Memory, MemoryPolicy, MemoryPolicyRule},
	providers::{EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig, Providers},
	ranking::{
		Ranking, RankingBlend, RankingBlendSegment, RankingDeterministic,
		RankingDeterministicDecay, RankingDeterministicHits, RankingDeterministicLexical,
		RankingDiversity, RankingRetrievalSources,
	},
	scopes::{ReadProfiles, ScopePrecedence, ScopeWriteAllowed, Scopes},
	search::{
		Search, SearchCache, SearchDynamic, SearchExpansion, SearchExplain, SearchGraphContext,
		SearchPrefilter, SearchRecursive,
	},
	security::{Security, SecurityAuthKey, SecurityAuthRole},
	service::Service,
	storage::{Postgres, Qdrant, Storage},
};

use serde::Deserialize;

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
