use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::docs::api::{
	selectors::{TextPositionSelector, TextQuoteSelector},
	trajectory::DocRetrievalTrajectory,
};

/// Request payload for excerpt retrieval.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsExcerptsGetRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Identifier of the source document.
	pub doc_id: Uuid,
	/// Excerpt budget level: `L0`, `L1`, or `L2`.
	pub level: String, // "L0" | "L1" | "L2"
	/// Optional chunk identifier when the caller already knows the chunk.
	pub chunk_id: Option<Uuid>,
	/// Optional quote-based selector.
	pub quote: Option<TextQuoteSelector>,
	/// Optional byte-position selector.
	pub position: Option<TextPositionSelector>,
	/// When true, includes retrieval trajectory output.
	pub explain: Option<bool>,
}

/// Verification metadata for one extracted excerpt.
#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptVerification {
	/// Whether the excerpt selectors verified against current content.
	pub verified: bool,
	/// Verification failure codes.
	pub verification_errors: Vec<String>,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// BLAKE3 hash of the returned excerpt.
	pub excerpt_hash: String,
}

/// Response payload for excerpt retrieval.
#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptResponse {
	/// Excerpt trace identifier.
	pub trace_id: Uuid,
	/// Identifier of the source document.
	pub doc_id: Uuid,
	/// Returned excerpt text.
	pub excerpt: String,
	/// Inclusive start offset of the returned window.
	pub start_offset: usize,
	/// Exclusive end offset of the returned window.
	pub end_offset: usize,
	/// Concrete selector resolution result.
	pub locator: DocsExcerptLocator,
	/// Verification metadata for the returned excerpt.
	pub verification: DocsExcerptVerification,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional retrieval trajectory emitted in explain mode.
	pub trajectory: Option<DocRetrievalTrajectory>,
}

/// Selector resolution metadata for an excerpt.
#[derive(Clone, Debug, Serialize)]
pub struct DocsExcerptLocator {
	/// Stable source span identifier for the matched selector span.
	pub span_id: Uuid,
	/// Selector kind that produced the match.
	pub selector_kind: String,
	/// Inclusive start offset of the matched selector span.
	pub match_start_offset: usize,
	/// Exclusive end offset of the matched selector span.
	pub match_end_offset: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Matched chunk identifier, when known.
	pub chunk_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Quote selector actually used for resolution.
	pub quote: Option<TextQuoteSelector>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Position selector actually used for resolution.
	pub position: Option<TextPositionSelector>,
}
