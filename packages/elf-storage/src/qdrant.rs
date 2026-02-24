use crate::Result;

pub const DENSE_VECTOR_NAME: &str = "dense";
pub const BM25_VECTOR_NAME: &str = "bm25";
pub const BM25_MODEL: &str = "qdrant/bm25";

pub struct QdrantStore {
	pub client: qdrant_client::Qdrant,
	pub collection: String,
	pub vector_dim: u32,
}
impl QdrantStore {
	pub fn new(cfg: &elf_config::Qdrant) -> Result<Self> {
		Self::new_with_collection(cfg, cfg.collection.as_str())
	}

	pub fn new_with_collection(cfg: &elf_config::Qdrant, collection: &str) -> Result<Self> {
		let client = qdrant_client::Qdrant::from_url(&cfg.url).build()?;

		Ok(Self { client, collection: collection.to_string(), vector_dim: cfg.vector_dim })
	}
}
