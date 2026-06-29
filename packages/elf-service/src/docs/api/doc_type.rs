use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Document classification used for persistence and retrieval filters.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
	/// Long-lived knowledge-base material.
	Knowledge,
	/// Chat transcripts or conversational context.
	Chat,
	/// Search-produced reference material.
	Search,
	/// Development-oriented artifacts such as code or plans.
	Dev,
}

impl DocType {
	/// Returns the canonical storage and API string for this document type.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Knowledge => "knowledge",
			Self::Chat => "chat",
			Self::Search => "search",
			Self::Dev => "dev",
		}
	}

	/// Parses a canonical document-type string.
	pub fn parse(raw_doc_type: &str) -> Result<Self> {
		match raw_doc_type {
			"knowledge" => Ok(Self::Knowledge),
			"chat" => Ok(Self::Chat),
			"search" => Ok(Self::Search),
			"dev" => Ok(Self::Dev),
			_ => Err(Error::InvalidRequest {
				message: "doc_type must be one of: knowledge, chat, search, dev.".to_string(),
			}),
		}
	}
}
