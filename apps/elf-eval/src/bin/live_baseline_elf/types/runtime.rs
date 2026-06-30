use crate::{BTreeMap, EmbeddingMode, PathBuf, Serialize, Uuid};

#[derive(Debug)]
pub(crate) struct BaselineRuntime {
	pub(crate) config_path: PathBuf,
	pub(crate) dsn: String,
	pub(crate) qdrant_url: String,
	pub(crate) collection: String,
	pub(crate) docs_collection: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct WorkerRunEvidence {
	pub(crate) label: String,
	pub(crate) expected_note_count: usize,
	pub(crate) concurrency: usize,
	pub(crate) iterations: usize,
	pub(crate) before: BTreeMap<String, i64>,
	pub(crate) after: BTreeMap<String, i64>,
	pub(crate) chunk_rows: i64,
	pub(crate) chunk_embedding_rows: i64,
	pub(crate) failed_jobs: Vec<FailedOutboxJob>,
}

#[derive(Debug, Serialize)]
pub(crate) struct FailedOutboxJob {
	pub(crate) note_id: Uuid,
	pub(crate) note_key: Option<String>,
	pub(crate) op: String,
	pub(crate) attempts: i32,
	pub(crate) last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct EmbeddingRuntimeReport {
	pub(crate) mode: EmbeddingMode,
	pub(crate) provider_id: String,
	pub(crate) model: String,
	pub(crate) dimensions: u32,
	pub(crate) timeout_ms: u64,
	pub(crate) api_base: String,
	pub(crate) path: String,
}
