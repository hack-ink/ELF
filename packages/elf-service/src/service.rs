use elf_config::Config;
use elf_storage::{db::Db, qdrant::QdrantStore};

use crate::Providers;

/// Main service container for ELF request handling.
pub struct ElfService {
	/// Repository configuration snapshot.
	pub cfg: Config,
	/// Postgres storage handle.
	pub db: Db,
	/// Qdrant storage handle.
	pub qdrant: QdrantStore,
	/// External model-provider adapters.
	pub providers: Providers,
}

impl ElfService {
	/// Builds a service with the default provider adapters.
	pub fn new(cfg: Config, db: Db, qdrant: QdrantStore) -> Self {
		Self { cfg, db, qdrant, providers: Providers::default() }
	}

	/// Builds a service with explicit provider adapters.
	pub fn with_providers(cfg: Config, db: Db, qdrant: QdrantStore, providers: Providers) -> Self {
		Self { cfg, db, qdrant, providers }
	}
}
