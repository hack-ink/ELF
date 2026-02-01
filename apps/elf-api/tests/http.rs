#[path = "../src/routes.rs"]
mod routes;
#[path = "../src/state.rs"]
mod state;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::ServiceExt;

#[tokio::test]
async fn health_ok() {
    let dsn = match std::env::var("ELF_TEST_PG_DSN") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Skipping health_ok; set ELF_TEST_PG_DSN to run this test.");
            return;
        }
    };
    let config = test_config(dsn);
    let state = state::AppState::new(config)
        .await
        .expect("Failed to initialize app state.");
    let app = routes::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("Failed to build request."),
        )
        .await
        .expect("Failed to call /health.");
    assert_eq!(response.status(), StatusCode::OK);
}

fn test_config(dsn: String) -> elf_config::Config {
    elf_config::Config {
        service: elf_config::Service {
            http_bind: "127.0.0.1:0".to_string(),
            admin_bind: "127.0.0.1:0".to_string(),
            log_level: "info".to_string(),
        },
        storage: elf_config::Storage {
            postgres: elf_config::Postgres {
                dsn,
                pool_max_conns: 1,
            },
            qdrant: elf_config::Qdrant {
                url: "http://127.0.0.1:6334".to_string(),
                collection: "elf_notes".to_string(),
                vector_dim: 3,
            },
        },
        providers: elf_config::Providers {
            embedding: dummy_provider(),
            rerank: dummy_provider(),
            llm_extractor: dummy_llm_provider(),
        },
        scopes: elf_config::Scopes {
            allowed: vec![
                "agent_private".to_string(),
                "project_shared".to_string(),
                "org_shared".to_string(),
            ],
            read_profiles: elf_config::ReadProfiles {
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
    }
}

fn dummy_provider() -> elf_config::ProviderConfig {
    elf_config::ProviderConfig {
        provider_id: "test".to_string(),
        base_url: "http://127.0.0.1:1".to_string(),
        api_key: "test-key".to_string(),
        path: "/".to_string(),
        model: "test".to_string(),
        timeout_ms: 1000,
        default_headers: serde_json::Map::new(),
    }
}

fn dummy_llm_provider() -> elf_config::LlmProviderConfig {
    elf_config::LlmProviderConfig {
        provider_id: "test".to_string(),
        base_url: "http://127.0.0.1:1".to_string(),
        api_key: "test-key".to_string(),
        path: "/".to_string(),
        model: "test".to_string(),
        temperature: 0.1,
        timeout_ms: 1000,
        default_headers: serde_json::Map::new(),
    }
}
