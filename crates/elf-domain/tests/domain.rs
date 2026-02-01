use elf_domain::cjk::contains_cjk;
use elf_domain::evidence::evidence_matches;
use elf_domain::ttl::compute_expires_at;

#[test]
fn detects_cjk() {
    assert!(contains_cjk("\u{4F60}\u{597D}"));
    assert!(!contains_cjk("hello"));
}

#[test]
fn evidence_requires_substring() {
    let messages = vec!["Hello world".to_string()];
    assert!(evidence_matches(&messages, 0, "world"));
    assert!(!evidence_matches(&messages, 0, "missing"));
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
            embedding: dummy_provider(),
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
        ranking: elf_config::Ranking {
            recency_tau_days: 60.0,
            tie_breaker_weight: 0.1,
        },
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
    };

    let now = time::OffsetDateTime::now_utc();
    let expires = compute_expires_at(None, "plan", &cfg, now).expect("TTL missing");
    assert!(expires > now);
}

fn dummy_provider() -> elf_config::ProviderConfig {
    elf_config::ProviderConfig {
        provider_id: "p".to_string(),
        base_url: "http://localhost".to_string(),
        api_key: "key".to_string(),
        path: "/".to_string(),
        model: "m".to_string(),
        timeout_ms: 1000,
        default_headers: serde_json::Map::new(),
    }
}

fn dummy_llm_provider() -> elf_config::LlmProviderConfig {
    elf_config::LlmProviderConfig {
        provider_id: "p".to_string(),
        base_url: "http://localhost".to_string(),
        api_key: "key".to_string(),
        path: "/".to_string(),
        model: "m".to_string(),
        temperature: 0.1,
        timeout_ms: 1000,
        default_headers: serde_json::Map::new(),
    }
}
