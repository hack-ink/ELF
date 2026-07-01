use time::OffsetDateTime;
use uuid::Uuid;

use elf_domain::consolidation::{
	ConsolidationInputRef, ConsolidationLineage, ConsolidationSourceKind,
	ConsolidationSourceSnapshot,
};

pub(in crate::acceptance::consolidation) fn source_ref(note_id: Uuid) -> ConsolidationInputRef {
	ConsolidationInputRef {
		kind: ConsolidationSourceKind::Note,
		id: note_id,
		snapshot: ConsolidationSourceSnapshot {
			status: Some("active".to_string()),
			updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			content_hash: Some("blake3:acceptance-source".to_string()),
			embedding_version: Some("test:test:4096".to_string()),
			trace_version: None,
			source_ref: serde_json::json!({ "schema": "acceptance/v1" }),
			metadata: serde_json::json!({ "fixture": "consolidation" }),
		},
	}
}

pub(in crate::acceptance::consolidation) fn lineage(
	source: &ConsolidationInputRef,
) -> ConsolidationLineage {
	ConsolidationLineage {
		source_refs: vec![source.clone()],
		parent_run_id: None,
		parent_proposal_ids: Vec::new(),
	}
}
