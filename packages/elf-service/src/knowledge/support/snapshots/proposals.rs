use crate::knowledge::support::{
	self, KnowledgeProposalSource, KnowledgeSourceKind, SourceSnapshot, serde_json,
};

pub(in crate::knowledge) fn proposal_source_snapshot(
	row: KnowledgeProposalSource,
) -> SourceSnapshot {
	let content_hash = support::hash_json_lossy(&serde_json::json!({
		"diff": row.diff.clone(),
		"proposed_payload": row.proposed_payload.clone(),
		"review_state": row.review_state.clone(),
	}));
	let line = format!("Applied proposal {}", row.proposal_kind);
	let snapshot = support::sanitize_proposal_snapshot(&serde_json::json!({
		"kind": "proposal",
		"proposal_id": row.proposal_id,
		"run_id": row.run_id,
		"agent_id": row.agent_id.clone(),
		"proposal_kind": row.proposal_kind.clone(),
		"apply_intent": row.apply_intent.clone(),
		"review_state": row.review_state.clone(),
		"source_refs": row.source_refs.clone(),
		"source_snapshot": row.source_snapshot.clone(),
		"lineage": row.lineage.clone(),
		"diff": row.diff.clone(),
		"confidence": row.confidence,
		"unsupported_claim_flags": row.unsupported_claim_flags.clone(),
		"contradiction_markers": row.contradiction_markers.clone(),
		"staleness_markers": row.staleness_markers.clone(),
		"target_ref": row.target_ref.clone(),
		"proposed_payload_hash": content_hash,
		"updated_at": row.updated_at,
	}));

	SourceSnapshot {
		kind: KnowledgeSourceKind::Proposal,
		id: row.proposal_id,
		status: Some(row.review_state),
		updated_at: Some(row.updated_at),
		content_hash: Some(content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "reviewed_proposal" }),
		line,
	}
}
