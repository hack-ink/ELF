use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use serde_json::{Map, Value};
use sqlx::PgPool;

use elf_config::{Config, EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_service::{
	AddNoteInput, AddNoteRequest, ElfService, EmbeddingProvider, ExtractorProvider, Providers,
	RerankProvider, ServiceError,
};
use elf_storage::{db::Db, qdrant::QdrantStore};

struct DummyEmbedding;

impl EmbeddingProvider for DummyEmbedding {
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> elf_service::BoxFuture<'a, color_eyre::Result<Vec<Vec<f32>>>> {
		let dim = (cfg.dimensions as usize).max(1);
		let vec = vec![0.0; dim];

		Box::pin(async move { Ok(vec![vec; texts.len()]) })
	}
}

struct DummyRerank;

impl RerankProvider for DummyRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> elf_service::BoxFuture<'a, color_eyre::Result<Vec<f32>>> {
		let scores = vec![0.0; docs.len()];

		Box::pin(async move { Ok(scores) })
	}
}

struct SpyExtractor {
	calls: Arc<AtomicUsize>,
}
impl SpyExtractor {
	fn new() -> Self {
		Self { calls: Arc::new(AtomicUsize::new(0)) }
	}

	fn count(&self) -> usize {
		self.calls.load(Ordering::SeqCst)
	}
}
impl ExtractorProvider for SpyExtractor {
	fn extract<'a>(
		&'a self,
		_cfg: &'a LlmProviderConfig,
		_messages: &'a [Value],
	) -> elf_service::BoxFuture<'a, color_eyre::Result<Value>> {
		self.calls.fetch_add(1, Ordering::SeqCst);

		Box::pin(async move { Ok(serde_json::json!({ "notes": [] })) })
	}
}

fn test_config() -> Config {
	Config {
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
				url: "http://localhost:6334".to_string(),
				collection: "mem_notes_v2".to_string(),
				vector_dim: 4_096,
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
				project_shared: false,
				org_shared: false,
			},
		},
		memory: elf_config::Memory {
			max_notes_per_add_event: 3,
			max_note_chars: 500,
			dup_sim_threshold: 0.9,
			update_sim_threshold: 0.8,
			candidate_k: 10,
			top_k: 5,
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
			},
			explain: elf_config::SearchExplain { retention_days: 7 },
		},
		ranking: elf_config::Ranking {
			recency_tau_days: 60.0,
			tie_breaker_weight: 0.1,
			blend: Default::default(),
		},
		lifecycle: elf_config::Lifecycle {
			ttl_days: elf_config::TtlDays {
				plan: 1,
				fact: 2,
				preference: 0,
				constraint: 0,
				decision: 0,
				profile: 0,
			},
			purge_deleted_after_days: 30,
			purge_deprecated_after_days: 180,
		},
		chunking: elf_config::Chunking {
			enabled: true,
			max_tokens: 512,
			overlap_tokens: 128,
			tokenizer_repo: None,
		},
		security: elf_config::Security {
			bind_localhost_only: true,
			reject_cjk: true,
			redact_secrets_on_write: true,
			evidence_min_quotes: 1,
			evidence_max_quotes: 2,
			evidence_max_quote_chars: 320,
			api_auth_token: None,
			admin_auth_token: None,
		},
		context: None,
		mcp: None,
	}
}

fn dummy_embedding_provider() -> elf_config::EmbeddingProviderConfig {
	elf_config::EmbeddingProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "3".to_string(),
		dimensions: 3,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn dummy_provider() -> elf_config::ProviderConfig {
	elf_config::ProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "3".to_string(),
		timeout_ms: 1_000,
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
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

#[tokio::test]
async fn add_note_does_not_call_llm() {
	let cfg = test_config();
	let pool =
		PgPool::connect_lazy(&cfg.storage.postgres.dsn).expect("Failed to create lazy pool.");
	let db = Db { pool };
	let qdrant = QdrantStore::new(&cfg.storage.qdrant).expect("Failed to create Qdrant store.");
	let spy = Arc::new(SpyExtractor::new());
	let providers = Providers::new(Arc::new(DummyEmbedding), Arc::new(DummyRerank), spy.clone());
	let service = ElfService::with_providers(cfg, db, qdrant, providers);
	let req = AddNoteRequest {
		tenant_id: "t1".to_string(),
		project_id: "p1".to_string(),
		agent_id: "a1".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			note_type: "fact".to_string(),
			key: None,
			text: "こんにちは".to_string(),
			importance: 0.5,
			confidence: 0.5,
			ttl_days: None,
			source_ref: serde_json::json!({}),
		}],
	};
	let result = service.add_note(req).await;

	assert!(matches!(result, Err(ServiceError::NonEnglishInput { .. })));

	assert_eq!(spy.count(), 0);
}

#[tokio::test]
async fn add_note_rejects_empty_notes() {
	let cfg = test_config();
	let pool =
		PgPool::connect_lazy(&cfg.storage.postgres.dsn).expect("Failed to create lazy pool.");
	let db = Db { pool };
	let qdrant = QdrantStore::new(&cfg.storage.qdrant).expect("Failed to create Qdrant store.");
	let spy = Arc::new(SpyExtractor::new());
	let providers = Providers::new(Arc::new(DummyEmbedding), Arc::new(DummyRerank), spy.clone());
	let service = ElfService::with_providers(cfg, db, qdrant, providers);
	let req = AddNoteRequest {
		tenant_id: "t1".to_string(),
		project_id: "p1".to_string(),
		agent_id: "a1".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![],
	};
	let result = service.add_note(req).await;

	assert!(matches!(result, Err(ServiceError::InvalidRequest { .. })));

	assert_eq!(spy.count(), 0);
}
