use crate::knowledge::{
	ConsolidationInputRef, ConsolidationSourceKind, ConsolidationSourceSnapshot,
	KnowledgePageResponse, KnowledgePageSectionResponse, KnowledgePageSourceRefResponse,
	KnowledgeSourceKind,
};

pub(in crate::knowledge::watch::candidates) fn consolidation_input_ref(
	source_ref: &KnowledgePageSourceRefResponse,
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Option<ConsolidationInputRef> {
	let kind = consolidation_source_kind(source_ref.source_kind.as_str())?;

	Some(ConsolidationInputRef {
		kind,
		id: source_ref.source_id,
		snapshot: ConsolidationSourceSnapshot {
			status: source_ref.source_status.clone(),
			updated_at: source_ref.source_updated_at,
			content_hash: source_ref.source_content_hash.clone(),
			embedding_version: None,
			trace_version: None,
			source_ref: source_ref.source_snapshot.clone(),
			metadata: serde_json::json!({
				"schema": "elf.knowledge_delta.source_ref/v1",
				"reason": reason,
				"page_id": page.page.page_id,
				"page_kind": page.page.page_kind,
				"page_key": page.page.page_key,
				"section_id": section.section_id,
				"section_key": section.section_key,
			}),
		},
	})
}

fn consolidation_source_kind(source_kind: &str) -> Option<ConsolidationSourceKind> {
	match KnowledgeSourceKind::parse(source_kind)? {
		KnowledgeSourceKind::Doc => Some(ConsolidationSourceKind::Doc),
		KnowledgeSourceKind::DocChunk => Some(ConsolidationSourceKind::DocChunk),
		KnowledgeSourceKind::Note => Some(ConsolidationSourceKind::Note),
		KnowledgeSourceKind::Event => Some(ConsolidationSourceKind::Event),
		KnowledgeSourceKind::Relation | KnowledgeSourceKind::Proposal => None,
	}
}
