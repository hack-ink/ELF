use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct KnowledgePageRebuildBody {
	pub(in crate::routes) page_kind: KnowledgePageKind,
	pub(in crate::routes) page_key: String,
	pub(in crate::routes) title: Option<String>,
	#[serde(default)]
	pub(in crate::routes) doc_ids: Vec<Uuid>,
	#[serde(default)]
	pub(in crate::routes) doc_chunk_ids: Vec<Uuid>,
	#[serde(default)]
	pub(in crate::routes) note_ids: Vec<Uuid>,
	#[serde(default)]
	pub(in crate::routes) event_ids: Vec<Uuid>,
	#[serde(default)]
	pub(in crate::routes) relation_ids: Vec<Uuid>,
	#[serde(default)]
	pub(in crate::routes) proposal_ids: Vec<Uuid>,
	#[serde(default = "empty_json_object")]
	pub(in crate::routes) provider_metadata: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct KnowledgePageChangedSourceBody {
	pub(in crate::routes) source_kind: KnowledgeSourceKind,
	pub(in crate::routes) source_id: Uuid,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct KnowledgePageWatchRebuildBody {
	pub(in crate::routes) changed_sources: Vec<KnowledgePageChangedSourceBody>,
	pub(in crate::routes) page_kind: Option<KnowledgePageKind>,
	pub(in crate::routes) limit: Option<u32>,
	pub(in crate::routes) generate_memory_candidates: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct KnowledgePagesListQuery {
	pub(in crate::routes) page_kind: Option<KnowledgePageKind>,
	pub(in crate::routes) limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct KnowledgePagesSearchBody {
	pub(in crate::routes) query: String,
	pub(in crate::routes) page_kind: Option<KnowledgePageKind>,
	pub(in crate::routes) limit: Option<u32>,
}
