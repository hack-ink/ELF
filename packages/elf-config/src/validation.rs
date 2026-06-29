mod chunking;
mod context;
mod mcp;
mod memory;
mod providers;
mod ranking;
mod search;
mod security;
mod service;
mod storage;

use crate::{Config, Result};

/// Validates a deserialized ELF configuration against repository runtime rules.
pub fn validate(cfg: &Config) -> Result<()> {
	security::validate(cfg)?;
	service::validate(cfg)?;
	storage::validate(cfg)?;
	providers::validate(cfg)?;
	memory::validate(cfg)?;
	search::validate(cfg)?;
	ranking::validate(cfg)?;
	chunking::validate(cfg)?;
	context::validate(cfg)?;
	mcp::validate(cfg)?;
	search::validate_graph_context(cfg)?;

	Ok(())
}
