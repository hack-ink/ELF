mod acceptance {
	mod chunking {
		pub use elf_chunking::ChunkingConfig;
	}

	mod add_note_no_llm;
	mod chunk_search;
	mod english_only_boundary;
	mod evidence_binding;
	mod idempotency;
	mod outbox_eventual_consistency;
	mod rebuild_qdrant;
	mod sot_vectors;

	use std::{
		env,
		sync::{
			Arc,
			atomic::{AtomicUsize, Ordering},
		},
	};

	use serde_json::{Map, Value};

	use elf_service::{
		ElfService, EmbeddingProvider, ExtractorProvider, Providers, RerankProvider,
	};
	use elf_storage::{db::Db, qdrant::QdrantStore};
	use elf_testkit::TestDatabase;

	pub fn test_qdrant_url() -> Option<String> {
		env::var("ELF_QDRANT_URL").ok()
	}

	pub async fn test_db() -> Option<elf_testkit::TestDatabase> {
		let base_dsn = elf_testkit::env_dsn()?;
		let db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");
		Some(db)
	}

	pub fn test_config(
		dsn: String,
		qdrant_url: String,
		vector_dim: u32,
		collection: String,
	) -> elf_config::Config {
		elf_config::Config {
			service: elf_config::Service {
				http_bind: "127.0.0.1:0".to_string(),
				mcp_bind: "127.0.0.1:0".to_string(),
				admin_bind: "127.0.0.1:0".to_string(),
				log_level: "info".to_string(),
			},
			storage: elf_config::Storage {
				postgres: elf_config::Postgres { dsn, pool_max_conns: 2 },
				qdrant: elf_config::Qdrant { url: qdrant_url, collection, vector_dim },
			},
			providers: elf_config::Providers {
				embedding: dummy_embedding_provider(),
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
			},
			context: None,
		}
	}

	pub async fn build_service(
		cfg: elf_config::Config,
		providers: Providers,
	) -> color_eyre::Result<ElfService> {
		let db = Db::connect(&cfg.storage.postgres).await?;
		db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;
		let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;
		Ok(ElfService::with_providers(cfg, db, qdrant, providers))
	}

	pub async fn reset_db(pool: &sqlx::PgPool) -> color_eyre::Result<()> {
		sqlx::query(
			"TRUNCATE memory_hits, memory_note_versions, note_chunk_embeddings, memory_note_chunks, \
	         note_embeddings, search_trace_items, search_traces, search_trace_outbox, search_sessions, \
	         indexing_outbox, memory_notes",
		)
		.execute(pool)
		.await?;
		Ok(())
	}

	pub struct StubEmbedding {
		pub vector_dim: u32,
	}

	impl EmbeddingProvider for StubEmbedding {
		fn embed<'a>(
			&'a self,
			_cfg: &'a elf_config::EmbeddingProviderConfig,
			texts: &'a [String],
		) -> elf_service::BoxFuture<'a, color_eyre::Result<Vec<Vec<f32>>>> {
			let dim = self.vector_dim as usize;
			let vectors = texts.iter().map(|_| vec![0.0; dim]).collect();
			Box::pin(async move { Ok(vectors) })
		}
	}

	pub struct SpyEmbedding {
		pub vector_dim: u32,
		pub calls: Arc<AtomicUsize>,
	}

	impl EmbeddingProvider for SpyEmbedding {
		fn embed<'a>(
			&'a self,
			_cfg: &'a elf_config::EmbeddingProviderConfig,
			texts: &'a [String],
		) -> elf_service::BoxFuture<'a, color_eyre::Result<Vec<Vec<f32>>>> {
			self.calls.fetch_add(1, Ordering::SeqCst);
			let dim = self.vector_dim as usize;
			let vectors = texts.iter().map(|_| vec![0.0; dim]).collect();
			Box::pin(async move { Ok(vectors) })
		}
	}

	pub struct StubRerank;

	impl RerankProvider for StubRerank {
		fn rerank<'a>(
			&'a self,
			_cfg: &'a elf_config::ProviderConfig,
			_query: &'a str,
			docs: &'a [String],
		) -> elf_service::BoxFuture<'a, color_eyre::Result<Vec<f32>>> {
			let scores = vec![0.5; docs.len()];
			Box::pin(async move { Ok(scores) })
		}
	}

	pub struct SpyExtractor {
		pub calls: Arc<AtomicUsize>,
		pub payload: Value,
	}

	impl ExtractorProvider for SpyExtractor {
		fn extract<'a>(
			&'a self,
			_cfg: &'a elf_config::LlmProviderConfig,
			_messages: &'a [Value],
		) -> elf_service::BoxFuture<'a, color_eyre::Result<Value>> {
			let payload = self.payload.clone();
			self.calls.fetch_add(1, Ordering::SeqCst);
			Box::pin(async move { Ok(payload) })
		}
	}

	pub fn dummy_embedding_provider() -> elf_config::EmbeddingProviderConfig {
		elf_config::EmbeddingProviderConfig {
			provider_id: "test".to_string(),
			api_base: "http://127.0.0.1:1".to_string(),
			api_key: "test-key".to_string(),
			path: "/".to_string(),
			model: "test".to_string(),
			dimensions: 3,
			timeout_ms: 1000,
			default_headers: Map::new(),
		}
	}

	pub fn dummy_provider() -> elf_config::ProviderConfig {
		elf_config::ProviderConfig {
			provider_id: "test".to_string(),
			api_base: "http://127.0.0.1:1".to_string(),
			api_key: "test-key".to_string(),
			path: "/".to_string(),
			model: "test".to_string(),
			timeout_ms: 1000,
			default_headers: Map::new(),
		}
	}

	pub fn dummy_llm_provider() -> elf_config::LlmProviderConfig {
		elf_config::LlmProviderConfig {
			provider_id: "test".to_string(),
			api_base: "http://127.0.0.1:1".to_string(),
			api_key: "test-key".to_string(),
			path: "/".to_string(),
			model: "test".to_string(),
			temperature: 0.1,
			timeout_ms: 1000,
			default_headers: Map::new(),
		}
	}
}
