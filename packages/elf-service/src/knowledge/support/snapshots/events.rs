use crate::knowledge::support::{
	self, KnowledgeEventSource, KnowledgeSourceKind, SourceSnapshot, serde_json,
};

pub(in crate::knowledge) fn event_source_snapshot(row: KnowledgeEventSource) -> SourceSnapshot {
	let content_hash = support::hash_json_lossy(&row.details);
	let line = format!(
		"add_event audit {} {} for {}{}",
		row.note_op,
		row.policy_decision,
		row.note_type,
		row.note_key.as_ref().map(|key| format!(" key {key}")).unwrap_or_default()
	);
	let snapshot = serde_json::json!({
		"kind": "event",
		"decision_id": row.decision_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"pipeline": row.pipeline.clone(),
		"note_type": row.note_type.clone(),
		"note_key": row.note_key.clone(),
		"note_id": row.note_id,
		"policy_decision": row.policy_decision.clone(),
		"note_op": row.note_op.clone(),
		"reason_code": row.reason_code.clone(),
		"details_hash": content_hash,
		"ts": row.ts,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Event,
		id: row.decision_id,
		status: Some(row.policy_decision),
		updated_at: Some(row.ts),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "event_audit" }),
		line,
	}
}
