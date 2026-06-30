use crate::knowledge::support::{
	self, KnowledgeRelationSource, KnowledgeSourceKind, SourceSnapshot, serde_json,
};

pub(in crate::knowledge) fn relation_source_snapshot(
	row: KnowledgeRelationSource,
) -> SourceSnapshot {
	let object = row.object_entity.clone().or(row.object_value.clone()).unwrap_or_default();
	let temporal_status = if row.valid_to.is_some() { "historical" } else { "current" };
	let line = format!("{} {} {} ({temporal_status}).", row.subject, row.predicate, object);
	let content_hash = support::hash_text(line.as_str());
	let snapshot = serde_json::json!({
		"kind": "relation",
		"fact_id": row.fact_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"subject": { "canonical": row.subject.clone(), "kind": row.subject_kind.clone() },
		"predicate": row.predicate.clone(),
		"object": {
			"entity": row.object_entity.clone(),
			"kind": row.object_kind.clone(),
			"value": row.object_value.clone()
		},
		"valid_from": row.valid_from,
		"valid_to": row.valid_to,
		"updated_at": row.updated_at,
		"content_hash": content_hash,
		"evidence_notes": row.evidence_notes.clone(),
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Relation,
		id: row.fact_id,
		status: Some(temporal_status.to_string()),
		updated_at: Some(row.updated_at),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "relation_fact" }),
		line,
	}
}
