use std::time::Duration;

use qdrant_client::{
	Qdrant, QdrantError,
	qdrant::{
		CreateCollectionBuilder, Distance, Modifier, SparseVectorParamsBuilder,
		SparseVectorsConfigBuilder, VectorParamsBuilder, VectorsConfigBuilder,
	},
};
use sqlx::PgExecutor;
use tokio::time;

use elf_config::Config;
use elf_service::{ElfService, Providers};
use elf_storage::{
	db::Db,
	qdrant::{BM25_VECTOR_NAME, DENSE_VECTOR_NAME, QdrantStore},
};
use elf_testkit::TestDatabase;

pub(crate) type AcceptanceResult<T> = Result<T, AcceptanceFailure>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AcceptanceFailure {
	#[error(transparent)]
	Storage(#[from] elf_storage::Error),
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),
	#[error(transparent)]
	Qdrant(#[from] QdrantError),
	#[error("{0}")]
	Message(String),
}

pub(crate) async fn test_db() -> Option<TestDatabase> {
	let base_dsn = elf_testkit::env_dsn()?;
	let db = TestDatabase::new(&base_dsn).await.expect("Failed to create test database.");

	Some(db)
}

pub(crate) async fn reset_qdrant_collection(
	client: &Qdrant,
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

pub(crate) async fn build_service(
	cfg: Config,
	providers: Providers,
) -> AcceptanceResult<ElfService> {
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	Ok(ElfService::with_providers(cfg, db, qdrant, providers))
}

pub(crate) async fn reset_db<'e, E>(executor: E) -> AcceptanceResult<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
TRUNCATE
	graph_entities,
	graph_entity_aliases,
	graph_predicates,
	graph_predicate_aliases,
	graph_facts,
	graph_fact_evidence,
	graph_fact_supersessions,
	memory_hits,
	memory_ingest_decisions,
	memory_note_versions,
	memory_space_grants,
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
	doc_indexing_outbox,
	doc_chunk_embeddings,
	doc_chunks,
	doc_documents,
	knowledge_page_lint_findings,
	knowledge_page_source_refs,
	knowledge_page_sections,
	knowledge_pages,
	consolidation_run_jobs,
	consolidation_proposal_reviews,
	consolidation_proposals,
	consolidation_runs,
	memory_notes",
	)
	.execute(executor)
	.await?;

	Ok(())
}
