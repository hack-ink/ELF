use clap::Parser;
use tracing_subscriber::EnvFilter;

pub mod worker;

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

	let state = worker::WorkerState { db, qdrant, embedding: config.providers.embedding };

	worker::run_worker(state).await
}
