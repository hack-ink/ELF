use elf_config::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, MemoryPolicy,
	MemoryPolicyRule, Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend,
	RankingBlendSegment, RankingDeterministic, RankingDeterministicDecay, RankingDeterministicHits,
	RankingDeterministicLexical, RankingDiversity, RankingRetrievalSources, ReadProfiles,
	ScopePrecedence, ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic,
	SearchExpansion, SearchExplain, SearchGraphContext, SearchPrefilter, SearchRecursive, Security,
	Service, Storage, TtlDays,
};

pub(crate) fn test_config(policy: MemoryPolicy) -> Config {
	let mut cfg = test_default_config();

	cfg.memory.policy = policy;

	cfg
}

fn test_default_config() -> Config {
	Config {
		service: test_service_config(),
		storage: test_storage_config(),
		providers: test_providers_config(),
		scopes: test_scopes_config(),
		memory: test_memory_config(),
		search: test_search_config(),
		ranking: test_ranking_config(),
		lifecycle: test_lifecycle_config(),
		security: test_security_config(),
		chunking: test_chunking_config(),
		context: None,
		mcp: None,
	}
}

fn test_service_config() -> Service {
	Service {
		http_bind: "127.0.0.1:8080".to_string(),
		mcp_bind: "127.0.0.1:8082".to_string(),
		admin_bind: "127.0.0.1:8081".to_string(),
		log_level: "info".to_string(),
	}
}

fn test_storage_config() -> Storage {
	Storage {
		postgres: Postgres {
			dsn: "postgres://user:pass@localhost/db".to_string(),
			pool_max_conns: 1,
		},
		qdrant: Qdrant {
			url: "http://localhost".to_string(),
			collection: "mem_notes_v2".to_string(),
			docs_collection: "doc_chunks_v1".to_string(),
			vector_dim: 4_096,
		},
	}
}

fn test_providers_config() -> Providers {
	Providers {
		embedding: test_embedding_provider_config(),
		rerank: test_rerank_provider_config(),
		llm_extractor: test_llm_extractor_provider_config(),
	}
}

fn test_embedding_provider_config() -> EmbeddingProviderConfig {
	EmbeddingProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		dimensions: 3,
		timeout_ms: 1_000,
		default_headers: Default::default(),
	}
}

fn test_rerank_provider_config() -> ProviderConfig {
	ProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		timeout_ms: 1_000,
		default_headers: Default::default(),
	}
}

fn test_llm_extractor_provider_config() -> LlmProviderConfig {
	LlmProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		temperature: 0.1,
		timeout_ms: 1_000,
		default_headers: Default::default(),
	}
}

fn test_scopes_config() -> Scopes {
	Scopes {
		allowed: vec!["agent_private".to_string()],
		read_profiles: test_read_profiles_config(),
		precedence: ScopePrecedence { agent_private: 30, project_shared: 20, org_shared: 10 },
		write_allowed: ScopeWriteAllowed {
			agent_private: true,
			project_shared: true,
			org_shared: true,
		},
	}
}

fn test_read_profiles_config() -> ReadProfiles {
	ReadProfiles {
		private_only: vec!["agent_private".to_string()],
		private_plus_project: vec!["agent_private".to_string()],
		all_scopes: vec!["agent_private".to_string()],
	}
}

fn test_memory_config() -> Memory {
	Memory {
		max_notes_per_add_event: 3,
		max_note_chars: 240,
		dup_sim_threshold: 0.92,
		update_sim_threshold: 0.85,
		candidate_k: 60,
		top_k: 12,
		policy: MemoryPolicy {
			rules: vec![
				MemoryPolicyRule {
					note_type: Some("fact".to_string()),
					scope: Some("agent_private".to_string()),
					min_confidence: Some(0.9),
					min_importance: Some(0.1),
				},
				MemoryPolicyRule {
					note_type: Some("preference".to_string()),
					scope: Some("agent_private".to_string()),
					min_confidence: Some(0.75),
					min_importance: None,
				},
				MemoryPolicyRule {
					note_type: Some("preference".to_string()),
					scope: None,
					min_confidence: Some(0.6),
					min_importance: None,
				},
				MemoryPolicyRule {
					note_type: None,
					scope: None,
					min_confidence: None,
					min_importance: None,
				},
			],
		},
	}
}

fn test_search_config() -> Search {
	Search {
		expansion: SearchExpansion {
			mode: "off".to_string(),
			max_queries: 4,
			include_original: true,
		},
		dynamic: SearchDynamic { min_candidates: 10, min_top_score: 0.12 },
		prefilter: SearchPrefilter { max_candidates: 0 },
		cache: SearchCache {
			enabled: true,
			expansion_ttl_days: 7,
			rerank_ttl_days: 7,
			max_payload_bytes: Some(262_144),
		},
		explain: SearchExplain {
			retention_days: 7,
			capture_candidates: false,
			candidate_retention_days: 2,
			write_mode: "outbox".to_string(),
		},
		recursive: SearchRecursive {
			enabled: false,
			max_depth: 2,
			max_children_per_node: 4,
			max_nodes_per_scope: 32,
			max_total_nodes: 256,
		},
		graph_context: SearchGraphContext {
			enabled: false,
			max_facts_per_item: 16,
			max_evidence_notes_per_fact: 16,
		},
	}
}

fn test_ranking_config() -> Ranking {
	Ranking {
		recency_tau_days: 60.0,
		tie_breaker_weight: 0.1,
		deterministic: test_ranking_deterministic_config(),
		blend: RankingBlend {
			enabled: true,
			rerank_normalization: "rank".to_string(),
			retrieval_normalization: "rank".to_string(),
			segments: vec![
				RankingBlendSegment { max_retrieval_rank: 3, retrieval_weight: 0.8 },
				RankingBlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.5 },
				RankingBlendSegment { max_retrieval_rank: 1_000_000, retrieval_weight: 0.2 },
			],
		},
		diversity: RankingDiversity {
			enabled: true,
			sim_threshold: 0.88,
			mmr_lambda: 0.7,
			max_skips: 64,
		},
		retrieval_sources: RankingRetrievalSources {
			fusion_weight: 1.0,
			structured_field_weight: 1.0,
			fusion_priority: 1,
			structured_field_priority: 0,
		},
	}
}

fn test_ranking_deterministic_config() -> RankingDeterministic {
	RankingDeterministic {
		enabled: false,
		lexical: RankingDeterministicLexical {
			enabled: false,
			weight: 0.05,
			min_ratio: 0.3,
			max_query_terms: 16,
			max_text_terms: 1_024,
		},
		hits: RankingDeterministicHits {
			enabled: false,
			weight: 0.05,
			half_saturation: 8.0,
			last_hit_tau_days: 14.0,
		},
		decay: RankingDeterministicDecay { enabled: false, weight: 0.05, tau_days: 30.0 },
	}
}

fn test_lifecycle_config() -> Lifecycle {
	Lifecycle {
		ttl_days: TtlDays {
			plan: 14,
			fact: 180,
			preference: 0,
			constraint: 0,
			decision: 0,
			profile: 0,
		},
		purge_deleted_after_days: 30,
		purge_deprecated_after_days: 180,
	}
}

fn test_security_config() -> Security {
	Security {
		bind_localhost_only: true,
		reject_non_english: true,
		redact_secrets_on_write: true,
		evidence_min_quotes: 1,
		evidence_max_quotes: 2,
		evidence_max_quote_chars: 320,
		auth_mode: "off".to_string(),
		auth_keys: vec![],
	}
}

fn test_chunking_config() -> Chunking {
	Chunking {
		enabled: true,
		max_tokens: 512,
		overlap_tokens: 128,
		tokenizer_repo: "REPLACE_ME".to_string(),
	}
}
