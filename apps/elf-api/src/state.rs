use std::sync::Arc;

use elf_service::ElfService;
use elf_storage::{db::Db, qdrant::QdrantStore};

#[derive(Clone)]
pub struct AppState {
	pub service: Arc<ElfService>,
}
impl AppState {
	pub async fn new(config: elf_config::Config) -> color_eyre::Result<Self> {
		let db = Db::connect(&config.storage.postgres).await?;

		db.ensure_schema(config.storage.qdrant.vector_dim).await?;

		let qdrant = QdrantStore::new(&config.storage.qdrant)?;
		let service = ElfService::new(config, db, qdrant);

		Ok(Self { service: Arc::new(service) })
	}
}
