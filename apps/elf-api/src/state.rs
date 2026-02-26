use std::sync::Arc;

use color_eyre::Result;

use elf_config::Config;
use elf_service::ElfService;
use elf_storage::{
	db::Db,
	qdrant::{DOCS_SEARCH_FILTER_INDEXES, QdrantStore},
};

#[derive(Clone)]
pub struct AppState {
	pub service: Arc<ElfService>,
}
impl AppState {
	pub async fn new(config: Config) -> Result<Self> {
		let db = Db::connect(&config.storage.postgres).await?;

		db.ensure_schema(config.storage.qdrant.vector_dim).await?;

		let qdrant = QdrantStore::new(&config.storage.qdrant)?;

		qdrant.ensure_collection().await?;

		let docs_qdrant = QdrantStore::new_with_collection(
			&config.storage.qdrant,
			&config.storage.qdrant.docs_collection,
		)?;

		docs_qdrant.ensure_collection().await?;
		docs_qdrant.ensure_payload_indexes(&DOCS_SEARCH_FILTER_INDEXES).await?;

		let service = ElfService::new(config, db, qdrant);

		Ok(Self { service: Arc::new(service) })
	}
}
