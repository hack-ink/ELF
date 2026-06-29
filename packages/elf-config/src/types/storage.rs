use serde::Deserialize;

/// Storage backend configuration for persisted note and document data.
#[derive(Debug, Deserialize)]
pub struct Storage {
	/// Postgres source-of-truth settings.
	pub postgres: Postgres,
	/// Qdrant derived-index settings.
	pub qdrant: Qdrant,
}

/// Postgres connection settings.
#[derive(Debug, Deserialize)]
pub struct Postgres {
	/// Postgres DSN used by ELF services.
	pub dsn: String,
	/// Maximum number of pooled Postgres connections.
	pub pool_max_conns: u32,
}

/// Qdrant collection settings for note and document vectors.
#[derive(Debug, Deserialize)]
pub struct Qdrant {
	/// Qdrant base URL used by clients in this workspace.
	pub url: String,
	/// Primary notes collection name.
	pub collection: String,
	/// Document-chunk collection name.
	pub docs_collection: String,
	/// Vector dimension expected by both note and document collections.
	pub vector_dim: u32,
}
