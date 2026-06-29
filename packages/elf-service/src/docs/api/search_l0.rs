use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{selectors::TextPositionSelector, trajectory::DocRetrievalTrajectory};

/// Request payload for L0 document retrieval.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsSearchL0Request {
	/// Tenant to search within.
	pub tenant_id: String,
	/// Project to search within.
	pub project_id: String,
	/// Agent used for access-control checks.
	pub caller_agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Search query text.
	pub query: String,
	/// Optional scope filter.
	pub scope: Option<String>,
	/// Optional status filter.
	pub status: Option<String>,
	/// Optional document-type filter.
	pub doc_type: Option<String>,
	/// Sparse-retrieval mode override.
	pub sparse_mode: Option<String>,
	/// Optional domain filter from source metadata.
	pub domain: Option<String>,
	/// Optional repository filter from source metadata.
	pub repo: Option<String>,
	/// Optional agent filter.
	pub agent_id: Option<String>,
	/// Optional thread filter.
	pub thread_id: Option<String>,
	/// Optional lower bound for `updated_at`.
	pub updated_after: Option<String>,
	/// Optional upper bound for `updated_at`.
	pub updated_before: Option<String>,
	/// Optional lower bound for source timestamp metadata.
	pub ts_gte: Option<String>,
	/// Optional upper bound for source timestamp metadata.
	pub ts_lte: Option<String>,
	/// Maximum number of returned items.
	pub top_k: Option<u32>,
	/// Retrieval breadth before deduplication and projection.
	pub candidate_k: Option<u32>,
	/// When true, includes retrieval trajectory output.
	pub explain: Option<bool>,
}

/// One chunk-level hit returned by `docs_search_l0`.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Item {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Stable pointer bundle for later excerpt or resolution workflows.
	pub pointer: DocsSearchL0ItemPointer,
	/// Final score after retrieval and boosting.
	pub score: f32,
	/// Returned snippet text.
	pub snippet: String,
	/// Scope key for the document.
	pub scope: String,
	/// Stored document type.
	pub doc_type: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent that ingested the document.
	pub agent_id: String,
	/// Last update timestamp for the document.
	pub updated_at: OffsetDateTime,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Chunk-level BLAKE3 hash.
	pub chunk_hash: String,
}

/// Response payload for `docs_search_l0`.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0Response {
	/// Retrieval trace identifier.
	pub trace_id: Uuid,
	/// Returned chunk hits.
	pub items: Vec<DocsSearchL0Item>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional retrieval trajectory emitted in explain mode.
	pub trajectory: Option<DocRetrievalTrajectory>,
}

/// Stable pointer for a chunk hit returned by document search.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemPointer {
	/// Pointer schema identifier.
	pub schema: String,
	/// Pointer resolver identifier.
	pub resolver: String,
	#[serde(rename = "ref")]
	/// Logical identifiers used by the resolver.
	pub reference: DocsSearchL0ItemReference,
	/// Freshness guard for the pointer target.
	pub state: DocsSearchL0ItemState,
	/// Hash aliases for simpler pointer consumers.
	pub hashes: DocsSearchL0ItemHashes,
	/// Selector hints that can hydrate this chunk through `docs_excerpts_get`.
	pub locator: DocsSearchL0ItemLocator,
}

/// Logical identifiers for a document-search hit.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemReference {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Stable source record identifier.
	pub source_record_id: Uuid,
	/// Stable source span identifier for this chunk.
	pub source_span_id: Uuid,
}

/// Freshness guard for a document-search hit.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemState {
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Chunk-level BLAKE3 hash.
	pub chunk_hash: String,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp for the document.
	pub doc_updated_at: OffsetDateTime,
}

/// Hash values carried with a document-search pointer.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemHashes {
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Chunk-level BLAKE3 hash.
	pub chunk_hash: String,
}

/// Locator hints carried with a document-search pointer.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSearchL0ItemLocator {
	/// Stable source span identifier for the locator.
	pub span_id: Uuid,
	/// Chunk byte position in the authoritative document content.
	pub position: TextPositionSelector,
}
