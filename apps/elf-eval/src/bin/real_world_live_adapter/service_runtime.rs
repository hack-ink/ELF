use super::*;

pub(super) async fn build_service(runtime: &BaselineRuntime) -> color_eyre::Result<ElfService> {
	let cfg = runtime_config(runtime)?;
	let vector_dim = cfg.storage.qdrant.vector_dim;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	Ok(ElfService::with_providers(cfg, db, qdrant, deterministic_providers(vector_dim)))
}

async fn build_worker_state(runtime: &BaselineRuntime) -> color_eyre::Result<WorkerState> {
	let cfg = runtime_config(runtime)?;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	let docs_qdrant =
		QdrantStore::new_with_collection(&cfg.storage.qdrant, &cfg.storage.qdrant.docs_collection)?;

	docs_qdrant.ensure_collection().await?;

	let tokenizer = elf_chunking::load_tokenizer(&cfg.chunking.tokenizer_repo)
		.map_err(|err| eyre::eyre!("Failed to load tokenizer for live adapter worker: {err}"))?;
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

pub(super) async fn run_worker(runtime: &BaselineRuntime) -> color_eyre::Result<()> {
	let state = Arc::new(build_worker_state(runtime).await?);

	for _ in 0..8 {
		let state = Arc::clone(&state);
		let mut set = JoinSet::new();

		set.spawn(async move {
			worker::process_once(&state)
				.await
				.map_err(|err| eyre::eyre!("Worker process_once failed: {err}"))
		});

		while let Some(joined) = set.join_next().await {
			joined??;
		}
	}

	Ok(())
}
