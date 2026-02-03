use elf_service::{EmbeddingProvider, ExtractorProvider, Providers, RerankProvider};
use sqlx::Connection;
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::const_new(());
const TEST_DB_LOCK_KEY: i64 = 0x454C4601;

pub fn test_dsn() -> Option<String> {
	std::env::var("ELF_PG_DSN").ok()
}

pub fn test_qdrant_url() -> Option<String> {
	std::env::var("ELF_QDRANT_URL").ok()
}

pub fn test_config(dsn: String, qdrant_url: String, vector_dim: u32) -> elf_config::Config {
	elf_config::Config {
		service: elf_config::Service {
			http_bind: "127.0.0.1:0".to_string(),
			mcp_bind: "127.0.0.1:0".to_string(),
			admin_bind: "127.0.0.1:0".to_string(),
			log_level: "info".to_string(),
		},
		storage: elf_config::Storage {
			postgres: elf_config::Postgres { dsn, pool_max_conns: 2 },
			qdrant: elf_config::Qdrant {
				url: qdrant_url,
				collection: "elf_acceptance".to_string(),
				vector_dim,
			},
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
	}
}

pub async fn build_service(
	cfg: elf_config::Config,
	providers: Providers,
) -> color_eyre::Result<elf_service::ElfService> {
	let db = elf_storage::db::Db::connect(&cfg.storage.postgres).await?;
	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;
	let qdrant = elf_storage::qdrant::QdrantStore::new(&cfg.storage.qdrant)?;
	Ok(elf_service::ElfService::with_providers(cfg, db, qdrant, providers))
}

pub struct DbLock {
	_guard: tokio::sync::MutexGuard<'static, ()>,
	_conn: sqlx::PgConnection,
}

pub async fn test_lock(dsn: &str) -> color_eyre::Result<DbLock> {
	let guard = TEST_LOCK.lock().await;
	let mut conn = sqlx::PgConnection::connect(dsn).await?;
	sqlx::query("SELECT pg_advisory_lock($1)").bind(TEST_DB_LOCK_KEY).execute(&mut conn).await?;
	Ok(DbLock { _guard: guard, _conn: conn })
}

pub async fn reset_db(pool: &sqlx::PgPool) -> color_eyre::Result<()> {
	sqlx::query(
		"TRUNCATE memory_hits, memory_note_versions, note_embeddings, indexing_outbox, memory_notes",
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
	pub payload: serde_json::Value,
}

impl ExtractorProvider for SpyExtractor {
	fn extract<'a>(
		&'a self,
		_cfg: &'a elf_config::LlmProviderConfig,
		_messages: &'a [serde_json::Value],
	) -> elf_service::BoxFuture<'a, color_eyre::Result<serde_json::Value>> {
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
		default_headers: serde_json::Map::new(),
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
		default_headers: serde_json::Map::new(),
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
		default_headers: serde_json::Map::new(),
	}
}

mod add_note_no_llm;
mod english_only_boundary;
mod evidence_binding;
mod idempotency;
mod outbox_eventual_consistency;
mod rebuild_qdrant;
mod sot_vectors;
