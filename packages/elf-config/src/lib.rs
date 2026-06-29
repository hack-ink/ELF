//! ELF configuration loading and validation.

mod error;
mod loader;
mod types;
mod validation;

pub use self::{
	error::{Error, Result},
	loader::load,
	types::{
		Chunking, Config, Context, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig,
		McpContext, Memory, MemoryPolicy, MemoryPolicyRule, Postgres, ProviderConfig, Providers,
		Qdrant, Ranking, RankingBlend, RankingBlendSegment, RankingDeterministic,
		RankingDeterministicDecay, RankingDeterministicHits, RankingDeterministicLexical,
		RankingDiversity, RankingRetrievalSources, ReadProfiles, ScopePrecedence,
		ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic, SearchExpansion,
		SearchExplain, SearchGraphContext, SearchPrefilter, SearchRecursive, Security,
		SecurityAuthKey, SecurityAuthRole, Service, Storage, TtlDays,
	},
	validation::validate,
};
