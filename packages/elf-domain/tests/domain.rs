use serde_json::Map;
use time::OffsetDateTime;

use elf_domain::{cjk, evidence, ttl};

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
fn computes_ttl_from_defaults() {
	let cfg = elf_config::Config {
		service: elf_config::Service {
			http_bind: "127.0.0.1:8080".to_string(),
			mcp_bind: "127.0.0.1:8082".to_string(),
			admin_bind: "127.0.0.1:8081".to_string(),
			log_level: "info".to_string(),
		},
		storage: elf_config::Storage {
			postgres: elf_config::Postgres {
				dsn: "postgres://user:pass@localhost/db".to_string(),
				pool_max_conns: 1,
			},
			qdrant: elf_config::Qdrant {
				url: "http://localhost".to_string(),
				collection: "mem_notes_v1".to_string(),
				vector_dim: 3,
			},
		},
		providers: elf_config::Providers {
			embedding: dummy_embedding_provider(),
			rerank: dummy_provider(),
			llm_extractor: dummy_llm_provider(),
		},
		scopes: elf_config::Scopes {
			allowed: vec!["agent_private".to_string()],
			read_profiles: elf_config::ReadProfiles {
				private_only: vec!["agent_private".to_string()],
				private_plus_project: vec!["agent_private".to_string()],
				all_scopes: vec!["agent_private".to_string()],
			},
			precedence: elf_config::ScopePrecedence {
				agent_private: 30,
				project_shared: 20,
				org_shared: 10,
			},
			write_allowed: elf_config::ScopeWriteAllowed {
				agent_private: true,
				project_shared: true,
				org_shared: true,
			},
		},
		memory: elf_config::Memory {
			max_notes_per_add_event: 3,
			max_note_chars: 240,
			dup_sim_threshold: 0.92,
			update_sim_threshold: 0.85,
			candidate_k: 60,
			top_k: 12,
		},
		search: elf_config::Search {
			expansion: elf_config::SearchExpansion {
				mode: "off".to_string(),
				max_queries: 4,
				include_original: true,
			},
			dynamic: elf_config::SearchDynamic { min_candidates: 10, min_top_score: 0.12 },
			prefilter: elf_config::SearchPrefilter { max_candidates: 0 },
			cache: elf_config::SearchCache {
				enabled: true,
				expansion_ttl_days: 7,
				rerank_ttl_days: 7,
				max_payload_bytes: Some(262_144),
				expansion_version: "v1".to_string(),
				rerank_version: "v1".to_string(),
			},
			explain: elf_config::SearchExplain { retention_days: 7 },
		},
		ranking: elf_config::Ranking { recency_tau_days: 60.0, tie_breaker_weight: 0.1 },
		lifecycle: elf_config::Lifecycle {
			ttl_days: elf_config::TtlDays {
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
		security: elf_config::Security {
			bind_localhost_only: true,
			reject_cjk: true,
			redact_secrets_on_write: true,
			evidence_min_quotes: 1,
			evidence_max_quotes: 2,
			evidence_max_quote_chars: 320,
		},
		chunking: elf_config::Chunking {
			enabled: true,
			max_tokens: 512,
			overlap_tokens: 128,
			tokenizer_repo: None,
		},
	};

	let now = OffsetDateTime::now_utc();
	let expires = ttl::compute_expires_at(None, "plan", &cfg, now).expect("TTL missing");
	assert!(expires > now);
}

fn dummy_embedding_provider() -> elf_config::EmbeddingProviderConfig {
	elf_config::EmbeddingProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		dimensions: 3,
		timeout_ms: 1000,
		default_headers: Map::new(),
	}
}

fn dummy_provider() -> elf_config::ProviderConfig {
	elf_config::ProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		timeout_ms: 1000,
		default_headers: Map::new(),
	}
}

fn dummy_llm_provider() -> elf_config::LlmProviderConfig {
	elf_config::LlmProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		temperature: 0.1,
		timeout_ms: 1000,
		default_headers: Map::new(),
	}
}
