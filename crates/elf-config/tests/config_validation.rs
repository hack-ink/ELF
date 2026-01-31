use elf_config::{
    Config,
    Lifecycle,
    LlmProviderConfig,
    Memory,
    Postgres,
    ProviderConfig,
    Providers,
    Qdrant,
    Ranking,
    ReadProfiles,
    ScopePrecedence,
    ScopeWriteAllowed,
    Scopes,
    Security,
    Service,
    Storage,
    TtlDays,
};

fn sample_config() -> Config {
    Config {
        service: Service {
            http_bind: "127.0.0.1:8080".to_string(),
            admin_bind: "127.0.0.1:8081".to_string(),
            log_level: "info".to_string(),
        },
        storage: Storage {
            postgres: Postgres {
                dsn: "postgres://user:pass@127.0.0.1:5432/elf".to_string(),
                pool_max_conns: 5,
            },
            qdrant: Qdrant {
                url: "http://127.0.0.1:6334".to_string(),
                collection: "mem_notes_v1".to_string(),
                vector_dim: 1536,
            },
        },
        providers: Providers {
            embedding: ProviderConfig {
                provider_id: "embed".to_string(),
                base_url: "http://localhost".to_string(),
                api_key: "key".to_string(),
                path: "/embeddings".to_string(),
                model: "model".to_string(),
                timeout_ms: 1000,
                default_headers: serde_json::Map::new(),
            },
            rerank: ProviderConfig {
                provider_id: "rerank".to_string(),
                base_url: "http://localhost".to_string(),
                api_key: "key".to_string(),
                path: "/rerank".to_string(),
                model: "model".to_string(),
                timeout_ms: 1000,
                default_headers: serde_json::Map::new(),
            },
            llm_extractor: LlmProviderConfig {
                provider_id: "llm".to_string(),
                base_url: "http://localhost".to_string(),
                api_key: "key".to_string(),
                path: "/chat/completions".to_string(),
                model: "model".to_string(),
                temperature: 0.1,
                timeout_ms: 1000,
                default_headers: serde_json::Map::new(),
            },
        },
        scopes: Scopes {
            allowed: vec!["agent_private".to_string()],
            read_profiles: ReadProfiles {
                private_only: vec!["agent_private".to_string()],
                private_plus_project: vec!["agent_private".to_string()],
                all_scopes: vec!["agent_private".to_string()],
            },
            precedence: ScopePrecedence {
                agent_private: 30,
                project_shared: 20,
                org_shared: 10,
            },
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
        },
        ranking: Ranking {
            recency_tau_days: 60.0,
            tie_breaker_weight: 0.1,
        },
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
        },
    }
}

#[test]
fn reject_cjk_must_be_true() {
    let mut cfg = sample_config();
    cfg.security.reject_cjk = false;
    assert!(elf_config::validate(&cfg).is_err());
}
