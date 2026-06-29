use super::*;

/// Request to rebuild one derived knowledge page from explicit source ids.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageRebuildRequest {
	/// Tenant that owns the page and source records.
	pub tenant_id: String,
	/// Project that owns the page and source records.
	pub project_id: String,
	/// Agent requesting the rebuild.
	pub agent_id: String,
	/// Page kind.
	pub page_kind: KnowledgePageKind,
	/// Stable page key within the tenant/project/kind namespace.
	pub page_key: String,
	/// Optional display title; a deterministic title is generated when omitted.
	pub title: Option<String>,
	#[serde(default)]
	/// Source Library documents to compile into the page.
	pub doc_ids: Vec<Uuid>,
	#[serde(default)]
	/// Source Library document chunks or spans to compile into the page.
	pub doc_chunk_ids: Vec<Uuid>,
	#[serde(default)]
	/// Memory note sources to compile into the page.
	pub note_ids: Vec<Uuid>,
	#[serde(default)]
	/// Durable add_event audit source ids to compile into the page.
	pub event_ids: Vec<Uuid>,
	#[serde(default)]
	/// Graph relation fact ids to compile into the page.
	pub relation_ids: Vec<Uuid>,
	#[serde(default)]
	/// Applied consolidation proposal ids to compile into the page.
	pub proposal_ids: Vec<Uuid>,
	#[serde(default = "empty_object")]
	/// Provider metadata for nondeterministic or future LLM-derived rebuilds.
	pub provider_metadata: Value,
}

/// Request to get one derived knowledge page.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageGetRequest {
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page identifier.
	pub page_id: Uuid,
}

/// Request to list derived knowledge pages.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePagesListRequest {
	/// Tenant that owns the pages.
	pub tenant_id: String,
	/// Project that owns the pages.
	pub project_id: String,
	/// Optional page-kind filter.
	pub page_kind: Option<KnowledgePageKind>,
	/// Maximum number of pages to return.
	pub limit: Option<u32>,
}

/// Request to lint one derived knowledge page against current source snapshots.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageLintRequest {
	/// Tenant that owns the page.
	pub tenant_id: String,
	/// Project that owns the page.
	pub project_id: String,
	/// Page identifier.
	pub page_id: Uuid,
}

/// Request to search derived knowledge page sections.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageSearchRequest {
	/// Tenant that owns the pages.
	pub tenant_id: String,
	/// Project that owns the pages.
	pub project_id: String,
	/// Agent requesting the page search.
	pub agent_id: String,
	/// Read profile controlling source visibility.
	pub read_profile: String,
	/// English-only query for page title, key, heading, or section content.
	pub query: String,
	/// Optional page-kind filter.
	pub page_kind: Option<KnowledgePageKind>,
	/// Maximum number of section snippets to return.
	pub limit: Option<u32>,
}

/// Request to rebuild pages affected by changed authoritative sources.
#[derive(Clone, Debug, Deserialize)]
pub struct KnowledgePageWatchRebuildRequest {
	/// Tenant that owns the pages and changed sources.
	pub tenant_id: String,
	/// Project that owns the pages and changed sources.
	pub project_id: String,
	/// Agent requesting the watch/rebuild operation.
	pub agent_id: String,
	/// Changed source references observed by a watcher or operator.
	pub changed_sources: Vec<KnowledgePageChangedSource>,
	/// Optional page-kind filter for the affected-page lookup.
	pub page_kind: Option<KnowledgePageKind>,
	/// Maximum number of affected pages to rebuild.
	pub limit: Option<u32>,
	#[serde(default = "default_generate_memory_candidates")]
	/// Whether changed knowledge deltas should queue reviewable memory proposals.
	pub generate_memory_candidates: bool,
}

/// Changed authoritative source reference for the watch/rebuild loop.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KnowledgePageChangedSource {
	/// Changed source kind.
	pub source_kind: KnowledgeSourceKind,
	/// Changed source identifier.
	pub source_id: Uuid,
}
