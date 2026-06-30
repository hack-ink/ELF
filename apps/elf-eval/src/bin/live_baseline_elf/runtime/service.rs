use color_eyre::Result;

use crate::{
	BaselineRuntime, ChunkingConfig, Db, ElfService, EmbeddingMode, QdrantStore, WorkerState, eyre,
};

pub(crate) async fn build_service(runtime: &BaselineRuntime) -> Result<ElfService> {
	let cfg = crate::runtime_config(runtime)?;
	let embedding_mode = crate::embedding_mode()?;
	let vector_dim = cfg.storage.qdrant.vector_dim;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	if embedding_mode == EmbeddingMode::Provider {
		Ok(ElfService::new(cfg, db, qdrant))
	} else {
		Ok(ElfService::with_providers(cfg, db, qdrant, crate::deterministic_providers(vector_dim)))
	}
}

pub(super) async fn build_worker_state(runtime: &BaselineRuntime) -> Result<WorkerState> {
	let cfg = crate::runtime_config(runtime)?;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	let docs_qdrant =
		QdrantStore::new_with_collection(&cfg.storage.qdrant, &cfg.storage.qdrant.docs_collection)?;

	docs_qdrant.ensure_collection().await?;

	let tokenizer = elf_chunking::load_tokenizer(&cfg.chunking.tokenizer_repo)
		.map_err(|err| eyre::eyre!("Failed to load tokenizer for live baseline worker: {err}"))?;
	let chunking = ChunkingConfig {
		max_tokens: cfg.chunking.max_tokens,
		overlap_tokens: cfg.chunking.overlap_tokens,
	};

	Ok(WorkerState {
		db,
		qdrant,
		docs_qdrant,
		embedding: cfg.providers.embedding,
		chunking,
		tokenizer,
	})
}
