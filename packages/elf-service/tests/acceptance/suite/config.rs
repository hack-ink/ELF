use std::{env, fs};

use ahash::AHashMap;
use serde_json::Map;
use tokenizers::{Tokenizer, models::wordlevel::WordLevel};

use elf_config::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, MemoryPolicy,
	Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend, RankingBlendSegment,
	RankingDeterministic, RankingDeterministicDecay, RankingDeterministicHits,
	RankingDeterministicLexical, RankingDiversity, RankingRetrievalSources, ReadProfiles,
	ScopePrecedence, ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic,
	SearchExpansion, SearchExplain, SearchGraphContext, SearchPrefilter, SearchRecursive, Security,
	Service, Storage, TtlDays,
};

pub(crate) fn test_qdrant_url() -> Option<String> {
	env::var("ELF_QDRANT_GRPC_URL").ok().or_else(|| env::var("ELF_QDRANT_URL").ok())
}

pub(crate) fn test_config(
	dsn: String,
	qdrant_url: String,
	vector_dim: u32,
	collection: String,
	docs_collection: String,
) -> Config {
	let mut embedding = dummy_embedding_provider();

	embedding.dimensions = vector_dim;

	Config {
		service: Service {
			http_bind: "127.0.0.1:0".to_string(),
			mcp_bind: "127.0.0.1:0".to_string(),
			admin_bind: "127.0.0.1:0".to_string(),
			log_level: "info".to_string(),
		},
		storage: Storage {
			postgres: Postgres { dsn, pool_max_conns: 2 },
			qdrant: Qdrant {
				url: qdrant_url,
				collection: collection.clone(),
				docs_collection,
				vector_dim,
			},
		},
		providers: Providers {
			embedding,
			rerank: dummy_provider(),
			llm_extractor: dummy_llm_provider(),
		},
		scopes: Scopes {
			allowed: vec![
				"agent_private".to_string(),
				"project_shared".to_string(),
				"org_shared".to_string(),
			],
			read_profiles: ReadProfiles {
				private_only: vec!["agent_private".to_string()],
				private_plus_project: vec![
					"agent_private".to_string(),
					"project_shared".to_string(),
				],
				all_scopes: vec![
					"agent_private".to_string(),
					"project_shared".to_string(),
					"org_shared".to_string(),
				],
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
			policy: MemoryPolicy { rules: vec![] },
		},
		search: test_search(),
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
		chunking: Chunking {
			enabled: true,
			max_tokens: 512,
			overlap_tokens: 128,
			tokenizer_repo: test_tokenizer_repo(&collection),
		},
		security: Security {
			bind_localhost_only: true,
			reject_non_english: true,
			redact_secrets_on_write: true,
			evidence_min_quotes: 1,
			evidence_max_quotes: 2,
			evidence_max_quote_chars: 320,
			auth_mode: "off".to_string(),
			auth_keys: vec![],
		},
		context: None,
		mcp: None,
	}
}

pub(crate) fn dummy_embedding_provider() -> EmbeddingProviderConfig {
	EmbeddingProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		dimensions: 4_096,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_provider() -> ProviderConfig {
	ProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_llm_provider() -> LlmProviderConfig {
	LlmProviderConfig {
		provider_id: "test".to_string(),
		api_base: "http://127.0.0.1:1".to_string(),
		api_key: "test-key".to_string(),
		path: "/".to_string(),
		model: "test".to_string(),
		temperature: 0.1,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn test_tokenizer_repo(collection: &str) -> String {
	let tokenizer_path = env::temp_dir().join(format!("{collection}-tokenizer.json"));

	if tokenizer_path.exists() {
		return tokenizer_path.to_string_lossy().into_owned();
	}

	let mut vocab = AHashMap::new();

	vocab.insert("<unk>".to_string(), 0_u32);

	let model = WordLevel::builder()
		.vocab(vocab)
		.unk_token("<unk>".to_string())
		.build()
		.expect("Failed to build acceptance tokenizer.");
	let tokenizer = Tokenizer::new(model);
	let parent = tokenizer_path.parent().expect("Temporary tokenizer path has a parent directory.");

	fs::create_dir_all(parent).expect("Failed to create acceptance tokenizer directory.");

	tokenizer.save(&tokenizer_path, false).expect("Failed to save acceptance tokenizer.");

	tokenizer_path.to_string_lossy().into_owned()
}

fn test_search() -> Search {
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
