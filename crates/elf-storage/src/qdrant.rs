use color_eyre::Result;
use qdrant_client::Qdrant;

pub struct QdrantStore {
    pub client: Qdrant,
    pub collection: String,
    pub vector_dim: u32,
}

impl QdrantStore {
    pub fn new(cfg: &elf_config::Qdrant) -> Result<Self> {
        let client = Qdrant::from_url(&cfg.url).build()?;
        Ok(Self {
            client,
            collection: cfg.collection.clone(),
            vector_dim: cfg.vector_dim,
        })
    }
}
