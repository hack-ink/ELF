use std::env;

/// Returns `ELF_PG_DSN` when it is available for integration tests.
pub fn env_dsn() -> Option<String> {
	env::var("ELF_PG_DSN").ok()
}

/// Returns the configured Qdrant URL for integration tests.
pub fn env_qdrant_url() -> Option<String> {
	env::var("ELF_QDRANT_GRPC_URL").or_else(|_| env::var("ELF_QDRANT_URL")).ok()
}
