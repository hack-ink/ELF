use super::PathBuf;

#[derive(Debug)]
pub(crate) struct LightragSource {
	pub(crate) evidence_id: String,
	pub(crate) file_source: String,
	pub(crate) artifact_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct BaselineRuntime {
	pub(crate) config_path: PathBuf,
	pub(crate) dsn: String,
	pub(crate) qdrant_url: String,
	pub(crate) collection: String,
	pub(crate) docs_collection: String,
}
