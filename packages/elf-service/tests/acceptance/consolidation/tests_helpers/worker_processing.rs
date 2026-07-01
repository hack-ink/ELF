use crate::acceptance;
use elf_chunking::ChunkingConfig;
use elf_service::ElfService;
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_worker::worker::{self, WorkerState};

pub(in crate::acceptance::consolidation) async fn process_consolidation_worker(
	service: &ElfService,
) {
	let tokenizer = elf_chunking::load_tokenizer(&service.cfg.chunking.tokenizer_repo)
		.expect("worker tokenizer should load");
	let mut embedding = acceptance::dummy_embedding_provider();

	embedding.dimensions = service.cfg.storage.qdrant.vector_dim;

	let worker_state = WorkerState {
		db: Db::connect(&service.cfg.storage.postgres).await.expect("Failed to connect worker DB."),
		qdrant: QdrantStore::new(&service.cfg.storage.qdrant)
			.expect("Failed to build Qdrant store."),
		docs_qdrant: QdrantStore::new_with_collection(
			&service.cfg.storage.qdrant,
			&service.cfg.storage.qdrant.docs_collection,
		)
		.expect("Failed to build docs Qdrant store."),
		embedding,
		chunking: ChunkingConfig {
			max_tokens: service.cfg.chunking.max_tokens,
			overlap_tokens: service.cfg.chunking.overlap_tokens,
		},
		tokenizer,
	};

	worker::process_once(&worker_state).await.expect("consolidation worker should process once");
}
