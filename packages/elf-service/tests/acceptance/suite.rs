mod add_note_no_llm;
mod chunk_search;
mod chunking;
mod english_only_boundary;
mod evidence_binding;
mod graph_ingestion;
mod idempotency;
mod outbox_eventual_consistency;
mod rebuild_qdrant;
mod sot_vectors;
mod structured_field_retrieval;

use std::{
	env,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::Duration,
};

use qdrant_client::{
	QdrantError,
	qdrant::{
		CreateCollectionBuilder, Distance, Modifier, SparseVectorParamsBuilder,
		SparseVectorsConfigBuilder, VectorParamsBuilder, VectorsConfigBuilder,
	},
};
use serde_json::{Map, Value};
use sqlx::PgExecutor;
use tokio::time;

use elf_config::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, Postgres,
	ProviderConfig, Ranking, RankingBlend, RankingBlendSegment, RankingDeterministic,
	RankingDeterministicDecay, RankingDeterministicHits, RankingDeterministicLexical,
	RankingDiversity, RankingRetrievalSources, ReadProfiles, ScopePrecedence, ScopeWriteAllowed,
	Scopes, Search, SearchCache, SearchDynamic, SearchExpansion, SearchExplain, SearchPrefilter,
	Security, Service, Storage, TtlDays,
};
use elf_service::{
	BoxFuture, ElfService, EmbeddingProvider, ExtractorProvider, RerankProvider, Result,
};
use elf_storage::{
	db::Db,
	qdrant::{BM25_VECTOR_NAME, DENSE_VECTOR_NAME, QdrantStore},
};
use elf_testkit::TestDatabase;

type AcceptanceResult<T> = Result<T, AcceptanceFailure>;

#[derive(Debug, thiserror::Error)]
enum AcceptanceFailure {
	#[error(transparent)]
	Storage(#[from] elf_storage::Error),
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),
	#[error(transparent)]
	Qdrant(#[from] QdrantError),
	#[error("{0}")]
	Message(String),
}

pub struct StubEmbedding {
	pub vector_dim: u32,
}
impl EmbeddingProvider for StubEmbedding {
	fn embed<'a>(
		&'a self,
		_cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
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
		_cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
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
		_cfg: &'a ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
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
		_cfg: &'a LlmProviderConfig,
		_messages: &'a [Value],
	) -> BoxFuture<'a, Result<Value>> {
		let payload = self.payload.clone();

		self.calls.fetch_add(1, Ordering::SeqCst);

		Box::pin(async move { Ok(payload) })
	}
}

pub fn test_qdrant_url() -> Option<String> {
	env::var("ELF_QDRANT_URL").ok()
}

pub fn test_config(dsn: String, qdrant_url: String, vector_dim: u32, collection: String) -> Config {
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
			qdrant: elf_config::Qdrant { url: qdrant_url, collection, vector_dim },
		},
		providers: elf_config::Providers {
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
		chunking: Chunking {
			enabled: true,
			max_tokens: 512,
			overlap_tokens: 128,
			tokenizer_repo: "gpt2".to_string(),
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
		context: None,
		mcp: None,
	}
}

pub fn dummy_embedding_provider() -> EmbeddingProviderConfig {
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

pub fn dummy_provider() -> ProviderConfig {
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

pub fn dummy_llm_provider() -> LlmProviderConfig {
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

pub async fn test_db() -> Option<TestDatabase> {
	let base_dsn = elf_testkit::env_dsn()?;
	let db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");

	Some(db)
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

async fn reset_qdrant_collection(
	client: &qdrant_client::Qdrant,
	collection: &str,
	vector_dim: u32,
) -> AcceptanceResult<()> {
	let max_attempts = 8;
	let mut backoff = Duration::from_millis(100);
	let mut last_err = None;

	for attempt in 1..=max_attempts {
		let _ = client.delete_collection(collection.to_string()).await;
		let mut vectors_config = VectorsConfigBuilder::default();

		vectors_config.add_named_vector_params(
			DENSE_VECTOR_NAME,
			VectorParamsBuilder::new(vector_dim.into(), Distance::Cosine),
		);

		let mut sparse_vectors_config = SparseVectorsConfigBuilder::default();

		sparse_vectors_config.add_named_vector_params(
			BM25_VECTOR_NAME,
			SparseVectorParamsBuilder::default().modifier(Modifier::Idf as i32),
		);

		let builder = CreateCollectionBuilder::new(collection.to_string())
			.vectors_config(vectors_config)
			.sparse_vectors_config(sparse_vectors_config);

		match client.create_collection(builder).await {
			Ok(_) => return Ok(()),
			Err(err) => {
				last_err = Some(err);

				if attempt == max_attempts {
					break;
				}

				time::sleep(backoff).await;

				backoff = backoff.saturating_mul(2).min(Duration::from_secs(2));
			},
		}
	}

	Err(AcceptanceFailure::Message(format!(
		"Failed to create Qdrant collection {collection:?} after {max_attempts} attempts: {last_err:?}."
	)))
}

async fn build_service(
	cfg: Config,
	providers: elf_service::Providers,
) -> AcceptanceResult<ElfService> {
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	Ok(ElfService::with_providers(cfg, db, qdrant, providers))
}

async fn reset_db<'e, E>(executor: E) -> AcceptanceResult<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
TRUNCATE
	graph_entities,
	graph_entity_aliases,
	graph_facts,
	graph_fact_evidence,
	memory_hits,
	memory_note_versions,
	note_field_embeddings,
	memory_note_fields,
	note_chunk_embeddings,
	memory_note_chunks,
	note_embeddings,
	search_trace_items,
	search_trace_stage_items,
	search_trace_stages,
	search_traces,
	search_trace_outbox,
	search_sessions,
	search_trace_candidates,
	indexing_outbox,
	memory_notes",
	)
	.execute(executor)
	.await?;

	Ok(())
}
