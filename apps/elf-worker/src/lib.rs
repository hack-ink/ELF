use clap::Parser;
use tokenizers::Tokenizer;
use tracing_subscriber::EnvFilter;

pub mod worker;
pub mod chunking;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	pub config: std::path::PathBuf,
}

pub async fn run(args: Args) -> color_eyre::Result<()> {
	let config = elf_config::load(&args.config)?;
	let filter = EnvFilter::new(config.service.log_level.clone());
	tracing_subscriber::fmt().with_env_filter(filter).init();

	let db = elf_storage::db::Db::connect(&config.storage.postgres).await?;
	db.ensure_schema(config.storage.qdrant.vector_dim).await?;
	let qdrant = elf_storage::qdrant::QdrantStore::new(&config.storage.qdrant)?;

	let tokenizer_repo = config
		.chunking
		.tokenizer_repo
		.clone()
		.unwrap_or_else(|| config.providers.embedding.model.clone());
	let tokenizer =
		Tokenizer::from_pretrained(tokenizer_repo, None).map_err(|err| color_eyre::eyre::eyre!(err))?;
	let chunking = chunking::ChunkingConfig {
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
