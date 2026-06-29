use super::{Parser, PathBuf, Subcommand};

#[derive(Debug, Parser)]
#[command(version = elf_cli::VERSION, rename_all = "kebab", styles = elf_cli::styles())]
pub(crate) struct Args {
	#[command(subcommand)]
	pub(crate) command: CommandArgs,
}

#[derive(Debug, Parser)]
pub(crate) struct ElfArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH")]
	pub(crate) fixtures: PathBuf,
	/// Directory where generated real_world_job fixtures are written.
	#[arg(long, value_name = "DIR")]
	pub(crate) out_fixtures: PathBuf,
	/// JSON evidence file for adapter setup/run/result details.
	#[arg(long, value_name = "FILE")]
	pub(crate) evidence_out: PathBuf,
	/// ELF config loaded before Docker runtime overrides are applied.
	#[arg(long, short = 'c', value_name = "FILE")]
	pub(crate) config: PathBuf,
	/// Adapter id embedded in generated adapter_response objects.
	#[arg(long, default_value = "elf_live_real_world")]
	pub(crate) adapter_id: String,
}

#[derive(Debug, Parser)]
pub(crate) struct QmdArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH")]
	pub(crate) fixtures: PathBuf,
	/// Directory where generated real_world_job fixtures are written.
	#[arg(long, value_name = "DIR")]
	pub(crate) out_fixtures: PathBuf,
	/// JSON evidence file for adapter setup/run/result details.
	#[arg(long, value_name = "FILE")]
	pub(crate) evidence_out: PathBuf,
	/// qmd checkout directory. The materializer clones into it when missing.
	#[arg(long, value_name = "DIR")]
	pub(crate) qmd_dir: PathBuf,
	/// Work directory for qmd home, corpus files, and command logs.
	#[arg(long, value_name = "DIR")]
	pub(crate) work_dir: PathBuf,
	/// qmd repository URL used when qmd_dir is absent.
	#[arg(long, default_value = "https://github.com/tobi/qmd.git")]
	pub(crate) qmd_repo_url: String,
	/// Adapter id embedded in generated adapter_response objects.
	#[arg(long, default_value = "qmd_live_real_world")]
	pub(crate) adapter_id: String,
}

#[derive(Debug, Parser)]
pub(crate) struct LightragArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH")]
	pub(crate) fixtures: PathBuf,
	/// Directory where generated real_world_job fixtures are written.
	#[arg(long, value_name = "DIR")]
	pub(crate) out_fixtures: PathBuf,
	/// JSON evidence file for adapter setup/run/result details.
	#[arg(long, value_name = "FILE")]
	pub(crate) evidence_out: PathBuf,
	/// Work directory for generated source files and command logs.
	#[arg(long, value_name = "DIR")]
	pub(crate) work_dir: PathBuf,
	/// LightRAG API base URL reachable from the Docker runner.
	#[arg(long, default_value = "http://lightrag:9621")]
	pub(crate) api_base: String,
	/// Optional LightRAG API bearer token.
	#[arg(long)]
	pub(crate) api_key: Option<String>,
	/// Adapter id embedded in generated adapter_response objects.
	#[arg(long, default_value = "lightrag_live_real_world")]
	pub(crate) adapter_id: String,
	/// LightRAG query mode used for context export.
	#[arg(long, default_value = "naive")]
	pub(crate) query_mode: String,
	/// Number of top results requested from LightRAG.
	#[arg(long, default_value_t = 5)]
	pub(crate) top_k: u32,
	/// Number of chunk results requested from LightRAG.
	#[arg(long, default_value_t = 5)]
	pub(crate) chunk_top_k: u32,
	/// Health-check attempts before returning a typed runtime failure.
	#[arg(long, default_value_t = 30)]
	pub(crate) startup_attempts: u32,
	/// Delay between LightRAG health-check attempts.
	#[arg(long, default_value_t = 2)]
	pub(crate) startup_interval_seconds: u64,
	/// Poll attempts for asynchronous document indexing.
	#[arg(long, default_value_t = 60)]
	pub(crate) index_attempts: u32,
	/// Delay between document indexing status checks.
	#[arg(long, default_value_t = 2)]
	pub(crate) index_interval_seconds: u64,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
pub(crate) enum CommandArgs {
	/// Materialize adapter responses by running jobs through ELF's service runtime.
	Elf(ElfArgs),
	/// Materialize adapter responses by running jobs through qmd's local CLI workflow.
	Qmd(QmdArgs),
	/// Materialize adapter responses by exporting LightRAG query context and source mappings.
	Lightrag(LightragArgs),
}
