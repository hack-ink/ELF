use serde_json::Map;
use time::OffsetDateTime;

use elf_config::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, MemoryPolicy,
	Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend, RankingBlendSegment,
	RankingDeterministic, RankingDeterministicDecay, RankingDeterministicHits,
	RankingDeterministicLexical, RankingDiversity, RankingRetrievalSources, ReadProfiles,
	ScopePrecedence, ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic,
	SearchExpansion, SearchExplain, SearchPrefilter, Security, Service, Storage, TtlDays,
};
use elf_domain::{cjk, evidence, ttl};

fn dummy_embedding_provider() -> EmbeddingProviderConfig {
	EmbeddingProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		dimensions: 3,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_provider() -> ProviderConfig {
	ProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_llm_provider() -> LlmProviderConfig {
	LlmProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		temperature: 0.1,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn test_ranking() -> Ranking {
	Ranking {
		recency_tau_days: 60.0,
		tie_breaker_weight: 0.1,
		deterministic: RankingDeterministic {
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
		},
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

fn base_config() -> Config {
	Config {
		service: Service {
			http_bind: "127.0.0.1:8080".to_string(),
			mcp_bind: "127.0.0.1:8082".to_string(),
			admin_bind: "127.0.0.1:8081".to_string(),
			log_level: "info".to_string(),
		},
		storage: Storage {
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
		},
		providers: Providers {
			embedding: dummy_embedding_provider(),
			rerank: dummy_provider(),
			llm_extractor: dummy_llm_provider(),
		},
		scopes: Scopes {
			allowed: vec!["agent_private".to_string()],
			read_profiles: ReadProfiles {
				private_only: vec!["agent_private".to_string()],
				private_plus_project: vec!["agent_private".to_string()],
				all_scopes: vec!["agent_private".to_string()],
			},
			precedence: ScopePrecedence { agent_private: 30, project_shared: 20, org_shared: 10 },
			write_allowed: ScopeWriteAllowed {
				agent_private: true,
				project_shared: true,
				org_shared: true,
			},
		},
		memory: Memory {
			max_notes_per_add_event: 3,
			max_note_chars: 240,
			dup_sim_threshold: 0.92,
			update_sim_threshold: 0.85,
			candidate_k: 60,
			top_k: 12,
			policy: MemoryPolicy::default(),
		},
		search: Search {
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
			recursive: Default::default(),
			graph_context: Default::default(),
		},
		ranking: test_ranking(),
		lifecycle: Lifecycle {
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
		},
		security: Security {
			bind_localhost_only: true,
			reject_cjk: true,
			redact_secrets_on_write: true,
			evidence_min_quotes: 1,
			evidence_max_quotes: 2,
			evidence_max_quote_chars: 320,
			auth_mode: "off".to_string(),
			auth_keys: vec![],
		},
		chunking: Chunking {
			enabled: true,
			max_tokens: 512,
			overlap_tokens: 128,
			tokenizer_repo: "REPLACE_ME".to_string(),
		},
		context: None,
		mcp: None,
	}
}

#[test]
fn detects_cjk() {
	assert!(cjk::contains_cjk("\u{4F60}\u{597D}"));
	assert!(!cjk::contains_cjk("hello"));
}

#[test]
fn evidence_requires_substring() {
	let messages = vec!["Hello world".to_string()];

	assert!(evidence::evidence_matches(&messages, 0, "world"));
	assert!(!evidence::evidence_matches(&messages, 0, "missing"));
}

#[test]
fn evidence_rejects_empty_quote() {
	let messages = vec!["Hello world".to_string()];

	assert!(!evidence::evidence_matches(&messages, 0, ""));
	assert!(!evidence::evidence_matches(&messages, 0, "   "));
}

#[test]
fn computes_ttl_from_defaults() {
	let cfg = base_config();
	let now = OffsetDateTime::now_utc();
	let expires = ttl::compute_expires_at(None, "plan", &cfg, now).expect("TTL missing");

	assert!(expires > now);
}
