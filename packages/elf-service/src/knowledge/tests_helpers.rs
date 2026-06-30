use std::collections::BTreeSet;

use serde_json::Value;

use crate::knowledge::{
	self, KnowledgeDeltaMemoryCandidate, KnowledgePage, KnowledgePageKind, KnowledgePageResponse,
	KnowledgePageSection, KnowledgePageSectionResponse, KnowledgePageSourceRef,
	KnowledgePageSourceRefResponse, KnowledgePageSummary, KnowledgeSourceKind, OffsetDateTime,
	SourceSnapshot, Uuid,
};

pub(super) fn test_source(kind: KnowledgeSourceKind, raw_id: u128, line: &str) -> SourceSnapshot {
	let id = Uuid::from_u128(raw_id);
	let content_hash = knowledge::hash_text(line);

	SourceSnapshot {
		kind,
		id,
		status: Some("active".to_string()),
		updated_at: Some(OffsetDateTime::UNIX_EPOCH),
		content_hash: Some(content_hash.clone()),
		snapshot: serde_json::json!({
			"kind": kind.as_str(),
			"id": id,
			"status": "active",
			"updated_at": OffsetDateTime::UNIX_EPOCH,
			"content_hash": content_hash,
		}),
		citation_metadata: serde_json::json!({ "fixture": "knowledge_unit" }),
		line: line.to_string(),
	}
}

pub(super) fn test_rebuild_request(
	page_kind: KnowledgePageKind,
) -> knowledge::KnowledgePageRebuildRequest {
	knowledge::KnowledgePageRebuildRequest {
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		agent_id: "agent".to_string(),
		page_kind,
		page_key: "elf".to_string(),
		title: Some("ELF".to_string()),
		doc_ids: Vec::new(),
		doc_chunk_ids: Vec::new(),
		note_ids: Vec::new(),
		event_ids: Vec::new(),
		relation_ids: Vec::new(),
		proposal_ids: Vec::new(),
		provider_metadata: knowledge::empty_object(),
	}
}

pub(super) fn test_page() -> KnowledgePage {
	KnowledgePage {
		page_id: Uuid::from_u128(1),
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		page_kind: "project".to_string(),
		page_key: "elf".to_string(),
		title: "ELF".to_string(),
		contract_schema: "elf.knowledge_page/v1".to_string(),
		status: "active".to_string(),
		rebuild_source_hash: "source-hash".to_string(),
		content_hash: "content-hash".to_string(),
		source_coverage: serde_json::json!({
			"source_count": 2,
			"cited_source_count": 1,
			"coverage_complete": false
		}),
		source_snapshot: serde_json::json!({}),
		rebuild_metadata: serde_json::json!({}),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		rebuilt_at: OffsetDateTime::UNIX_EPOCH,
	}
}

pub(super) fn test_section(
	section_id: Uuid,
	section_key: &str,
	citations: Value,
	unsupported_reason: Option<String>,
) -> KnowledgePageSection {
	KnowledgePageSection {
		section_id,
		page_id: Uuid::from_u128(1),
		section_key: section_key.to_string(),
		heading: section_key.to_string(),
		role: "current_truth".to_string(),
		content: "Section content.".to_string(),
		ordinal: 0,
		citations,
		unsupported_reason,
		content_hash: "section-hash".to_string(),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
	}
}

pub(super) fn test_source_ref(section_id: Uuid) -> KnowledgePageSourceRef {
	test_source_ref_for(section_id, Uuid::from_u128(31), "source-hash")
}

pub(super) fn test_source_ref_for(
	section_id: Uuid,
	source_id: Uuid,
	source_hash: &str,
) -> KnowledgePageSourceRef {
	KnowledgePageSourceRef {
		ref_id: Uuid::from_u128(30),
		page_id: Uuid::from_u128(21),
		section_id: Some(section_id),
		source_kind: "note".to_string(),
		source_id,
		source_status: Some("active".to_string()),
		source_updated_at: Some(OffsetDateTime::UNIX_EPOCH),
		source_content_hash: Some(source_hash.to_string()),
		source_snapshot: serde_json::json!({
			"schema": "test_source/v1",
			"source_id": source_id,
			"content_hash": source_hash,
		}),
		citation_metadata: serde_json::json!({}),
		created_at: OffsetDateTime::UNIX_EPOCH,
	}
}

pub(super) fn current_source_keys_for(source_refs: &[&KnowledgePageSourceRef]) -> BTreeSet<String> {
	source_refs
		.iter()
		.map(|source_ref| {
			knowledge::current_key(source_ref.source_kind.as_str(), source_ref.source_id)
		})
		.collect()
}

pub(super) fn test_page_response(section_id: Uuid, source_id: Uuid) -> KnowledgePageResponse {
	let page = test_page();
	let section = test_section(
		section_id,
		"source-notes",
		serde_json::json!([{ "source_kind": "note", "source_id": source_id }]),
		None,
	);
	let source_ref = test_source_ref_for(section_id, source_id, "new-hash");

	KnowledgePageResponse {
		page: KnowledgePageSummary::from(page),
		sections: vec![KnowledgePageSectionResponse {
			citation_count: 1,
			source_ref_count: 1,
			coverage_complete: true,
			source_backlinks: Vec::new(),
			..KnowledgePageSectionResponse::from(section)
		}],
		source_refs: vec![KnowledgePageSourceRefResponse::from(source_ref)],
		lint_findings: Vec::new(),
	}
}

pub(super) fn assert_candidate_is_reviewable(candidate: &KnowledgeDeltaMemoryCandidate) {
	assert_eq!(candidate.reason, "changed_claim");
	assert_eq!(candidate.source_refs.len(), 1);
	assert_eq!(candidate.source_refs[0].kind.as_str(), "note");
	assert_eq!(candidate.source_snapshot["source_mutation_allowed"], false);
	assert_eq!(candidate.diff.after["reason"], "changed_claim");
	assert_eq!(candidate.proposed_payload["type"], "plan");
	assert_eq!(candidate.proposed_payload["source_ref"]["schema"], "elf.knowledge_delta/v1");
}
