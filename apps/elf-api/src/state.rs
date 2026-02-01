use std::sync::Arc;

use serde::Serialize;

pub type AddNoteRequest = serde_json::Value;
pub type AddNoteResponse = serde_json::Value;
pub type AddEventRequest = serde_json::Value;
pub type AddEventResponse = serde_json::Value;
pub type SearchRequest = serde_json::Value;
pub type SearchResponse = serde_json::Value;
pub type ListResponse = serde_json::Value;
pub type UpdateRequest = serde_json::Value;
pub type UpdateResponse = serde_json::Value;
pub type DeleteRequest = serde_json::Value;
pub type DeleteResponse = serde_json::Value;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<elf_config::Config>,
    pub service: Arc<ElfService>,
}

impl AppState {
    pub fn new(config: elf_config::Config) -> Self {
        let config = Arc::new(config);
        let service = Arc::new(ElfService::new(config.clone()));
        Self { config, service }
    }
}

#[derive(Clone)]
pub struct ElfService {
    #[allow(dead_code)]
    config: Arc<elf_config::Config>,
    #[allow(dead_code)]
    placeholder: elf_service::Placeholder,
}

impl ElfService {
    pub fn new(config: Arc<elf_config::Config>) -> Self {
        Self {
            config,
            placeholder: elf_service::Placeholder,
        }
    }

    pub async fn add_note(&self, _req: AddNoteRequest) -> Result<AddNoteResponse, ServiceError> {
        // TODO: Enforce English-only boundary once request types are defined.
        Err(ServiceError::not_implemented("add_note"))
    }

    pub async fn add_event(
        &self,
        _req: AddEventRequest,
    ) -> Result<AddEventResponse, ServiceError> {
        // TODO: Enforce English-only boundary once request types are defined.
        Err(ServiceError::not_implemented("add_event"))
    }

    pub async fn search(&self, _req: SearchRequest) -> Result<SearchResponse, ServiceError> {
        // TODO: Enforce English-only boundary once request types are defined.
        Err(ServiceError::not_implemented("search"))
    }

    pub async fn list(&self) -> Result<ListResponse, ServiceError> {
        Err(ServiceError::not_implemented("list"))
    }

    pub async fn update(&self, _req: UpdateRequest) -> Result<UpdateResponse, ServiceError> {
        // TODO: Enforce English-only boundary once request types are defined.
        Err(ServiceError::not_implemented("update"))
    }

    pub async fn delete(&self, _req: DeleteRequest) -> Result<DeleteResponse, ServiceError> {
        Err(ServiceError::not_implemented("delete"))
    }

    pub async fn rebuild_qdrant(&self) -> Result<RebuildReport, ServiceError> {
        Err(ServiceError::not_implemented("rebuild_qdrant"))
    }
}

#[derive(Debug)]
pub enum ServiceError {
    NotImplemented { operation: &'static str },
}

impl ServiceError {
    pub fn not_implemented(operation: &'static str) -> Self {
        Self::NotImplemented { operation }
    }
}

#[derive(Debug, Serialize)]
pub struct RebuildReport {
    pub rebuilt_count: u64,
    pub missing_vector_count: u64,
    pub error_count: u64,
}

#[cfg(test)]
pub fn test_state() -> AppState {
    AppState::new(test_config())
}

#[cfg(test)]
fn test_config() -> elf_config::Config {
    elf_config::Config {
        service: elf_config::Service {
            http_bind: "127.0.0.1:0".to_string(),
            admin_bind: "127.0.0.1:0".to_string(),
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
                private_only: vec![],
                private_plus_project: vec![],
                all_scopes: vec![],
            },
            precedence: elf_config::ScopePrecedence {
                agent_private: 1,
                project_shared: 2,
                org_shared: 3,
            },
            write_allowed: elf_config::ScopeWriteAllowed {
                agent_private: true,
                project_shared: true,
                org_shared: true,
            },
        },
        memory: elf_config::Memory {
            max_notes_per_add_event: 3,
            max_note_chars: 1000,
            dup_sim_threshold: 0.9,
            update_sim_threshold: 0.95,
            candidate_k: 20,
            top_k: 5,
        },
        ranking: elf_config::Ranking {
            recency_tau_days: 7.0,
            tie_breaker_weight: 0.2,
        },
        lifecycle: elf_config::Lifecycle {
            ttl_days: elf_config::TtlDays {
                plan: 1,
                fact: 1,
                preference: 1,
                constraint: 1,
                decision: 1,
                profile: 1,
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

#[cfg(test)]
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

#[cfg(test)]
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
