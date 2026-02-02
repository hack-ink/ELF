use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
	pub service: Arc<elf_service::ElfService>,
}

impl AppState {
	pub async fn new(config: elf_config::Config) -> color_eyre::Result<Self> {
		let db = elf_storage::db::Db::connect(&config.storage.postgres).await?;
		db.ensure_schema(config.storage.qdrant.vector_dim).await?;
		let qdrant = elf_storage::qdrant::QdrantStore::new(&config.storage.qdrant)?;
		let service = elf_service::ElfService::new(config, db, qdrant);
		Ok(Self { service: Arc::new(service) })
	}
}
