use crate::acceptance::knowledge_pages::helpers::{
	AGENT_ID, KnowledgeSourceIds, PROJECT_ID, TENANT_ID,
};
use elf_domain::knowledge::KnowledgePageKind;
use elf_service::KnowledgePageRebuildRequest;

pub(crate) fn knowledge_foundation_request(ids: KnowledgeSourceIds) -> KnowledgePageRebuildRequest {
	KnowledgePageRebuildRequest {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		agent_id: AGENT_ID.to_string(),
		page_kind: KnowledgePageKind::Project,
		page_key: "knowledge-foundation".to_string(),
		title: Some("Knowledge Foundation".to_string()),
		doc_ids: vec![ids.doc_id],
		doc_chunk_ids: vec![ids.chunk_id],
		note_ids: vec![ids.note_id],
		event_ids: vec![ids.event_id],
		relation_ids: vec![ids.fact_id],
		proposal_ids: vec![ids.proposal_id],
		provider_metadata: serde_json::json!({}),
	}
}
