use crate::knowledge::support::{
	self, KnowledgeNoteSource, KnowledgeSourceKind, SourceSnapshot, serde_json,
};

pub(in crate::knowledge) fn note_source_snapshot(row: KnowledgeNoteSource) -> SourceSnapshot {
	let content_hash = support::hash_text(row.text.as_str());
	let line = format!("{}{}", support::note_prefix(&row), row.text);
	let snapshot = serde_json::json!({
		"kind": "note",
		"note_id": row.note_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"type": row.note_type.clone(),
		"key": row.key.clone(),
		"status": row.status.clone(),
		"updated_at": row.updated_at,
		"created_at": row.created_at,
		"expires_at": row.expires_at,
		"embedding_version": row.embedding_version.clone(),
		"content_hash": content_hash,
		"source_ref": row.source_ref.clone(),
		"importance": row.importance,
		"confidence": row.confidence,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Note,
		id: row.note_id,
		status: Some(row.status),
		updated_at: Some(row.updated_at),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "source_note" }),
		line,
	}
}
