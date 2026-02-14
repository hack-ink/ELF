pub mod worker;

mod error;

pub use error::{Error, Result};

use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use elf_chunking::ChunkingConfig;
use elf_storage::{db::Db, qdrant::QdrantStore};

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	pub config: PathBuf,
}

pub async fn run(args: Args) -> Result<()> {
	let config = elf_config::load(&args.config).map_err(|err| Error::Message(err.to_string()))?;
	let filter = EnvFilter::new(config.service.log_level.clone());

	tracing_subscriber::fmt().with_env_filter(filter).init();

	let db = Db::connect(&config.storage.postgres).await?;

	db.ensure_schema(config.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&config.storage.qdrant)?;
	let tokenizer_repo = config
		.chunking
		.tokenizer_repo
		.clone()
		.unwrap_or_else(|| config.providers.embedding.model.clone());
	let tokenizer = elf_chunking::load_tokenizer(&tokenizer_repo)?;
	let chunking = ChunkingConfig {
		max_tokens: config.chunking.max_tokens,
		overlap_tokens: config.chunking.overlap_tokens,
	};
	let state = worker::WorkerState {
		db,
		qdrant,
		embedding: config.providers.embedding,
		chunking,
		tokenizer,
	};

	worker::run_worker(state).await
}
