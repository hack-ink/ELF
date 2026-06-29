use super::*;

pub(in crate::knowledge) fn doc_source_snapshot(row: KnowledgeDocSource) -> SourceSnapshot {
	let title = row.title.clone().unwrap_or_else(|| "Untitled source document".to_string());
	let excerpt = truncate_chars(normalize_whitespace(row.content.as_str()).as_str(), 240);
	let line = format!("[doc:{}] {title}: {excerpt}", row.doc_type);
	let snapshot = serde_json::json!({
		"kind": "doc",
		"doc_id": row.doc_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"doc_type": row.doc_type.clone(),
		"status": row.status.clone(),
		"title": row.title.clone(),
		"content_bytes": row.content_bytes,
		"content_hash": row.content_hash.clone(),
		"source_ref": row.source_ref.clone(),
		"created_at": row.created_at,
		"updated_at": row.updated_at,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::Doc,
		id: row.doc_id,
		status: Some(row.status),
		updated_at: Some(row.updated_at),
		content_hash: Some(row.content_hash),
		snapshot,
		citation_metadata: serde_json::json!({ "section_role": "source_document" }),
		line,
	}
}

pub(in crate::knowledge) fn doc_chunk_source_snapshot(
	row: KnowledgeDocChunkSource,
) -> SourceSnapshot {
	let title = row.title.clone().unwrap_or_else(|| "Untitled source document".to_string());
	let excerpt = truncate_chars(normalize_whitespace(row.chunk_text.as_str()).as_str(), 240);
	let span_id = source_span_id(
		row.doc_content_hash.as_str(),
		row.start_offset.max(0) as usize,
		row.end_offset.max(row.start_offset).max(0) as usize,
		"captured",
	);
	let line = format!(
		"[doc_chunk:{}:{}-{}] {title}: {excerpt}",
		row.chunk_index, row.start_offset, row.end_offset
	);
	let source_span = serde_json::json!({
		"schema": "doc_source_span/v1",
		"span_id": span_id,
		"chunk_id": row.chunk_id,
		"status": "captured",
		"reason_code": null,
		"start_offset": row.start_offset,
		"end_offset": row.end_offset,
		"content_hash": row.doc_content_hash.clone(),
		"chunk_hash": row.chunk_hash.clone(),
	});
	let snapshot = serde_json::json!({
		"kind": "doc_chunk",
		"chunk_id": row.chunk_id,
		"doc_id": row.doc_id,
		"agent_id": row.agent_id.clone(),
		"scope": row.scope.clone(),
		"doc_type": row.doc_type.clone(),
		"status": row.status.clone(),
		"title": row.title.clone(),
		"source_ref": row.source_ref.clone(),
		"doc_content_hash": row.doc_content_hash.clone(),
		"doc_updated_at": row.doc_updated_at,
		"chunk_index": row.chunk_index,
		"start_offset": row.start_offset,
		"end_offset": row.end_offset,
		"chunk_hash": row.chunk_hash.clone(),
		"chunk_created_at": row.chunk_created_at,
		"source_span": source_span,
	});

	SourceSnapshot {
		kind: KnowledgeSourceKind::DocChunk,
		id: row.chunk_id,
		status: Some(row.status),
		updated_at: Some(row.doc_updated_at),
		content_hash: Some(row.chunk_hash),
		snapshot,
		citation_metadata: serde_json::json!({
			"section_role": "source_span",
			"doc_id": row.doc_id,
			"span_id": span_id,
			"start_offset": row.start_offset,
			"end_offset": row.end_offset,
		}),
		line,
	}
}

pub(in crate::knowledge) fn note_source_snapshot(row: KnowledgeNoteSource) -> SourceSnapshot {
	let content_hash = hash_text(row.text.as_str());
	let line = format!("{}{}", note_prefix(&row), row.text);
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

pub(in crate::knowledge) fn event_source_snapshot(row: KnowledgeEventSource) -> SourceSnapshot {
	let content_hash = hash_json_lossy(&row.details);
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

pub(in crate::knowledge) fn relation_source_snapshot(
	row: KnowledgeRelationSource,
) -> SourceSnapshot {
	let object = row.object_entity.clone().or(row.object_value.clone()).unwrap_or_default();
	let temporal_status = if row.valid_to.is_some() { "historical" } else { "current" };
	let line = format!("{} {} {} ({temporal_status}).", row.subject, row.predicate, object);
	let content_hash = hash_text(line.as_str());
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

pub(in crate::knowledge) fn proposal_source_snapshot(
	row: KnowledgeProposalSource,
) -> SourceSnapshot {
	let content_hash = hash_json_lossy(&serde_json::json!({
		"diff": row.diff.clone(),
		"proposed_payload": row.proposed_payload.clone(),
		"review_state": row.review_state.clone(),
	}));
	let line = format!("Applied proposal {}", row.proposal_kind);
	let snapshot = sanitize_proposal_snapshot(&serde_json::json!({
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

pub(in crate::knowledge) fn sanitize_proposal_snapshot(source_snapshot: &Value) -> Value {
	let Some(object) = source_snapshot.as_object() else {
		return serde_json::json!({
			"kind": "proposal",
			"sanitized": true,
			"source_visibility": "proposal_metadata_only",
		});
	};
	let nested_source_count =
		object.get("source_refs").and_then(Value::as_array).map(Vec::len).unwrap_or_default();
	let mut sanitized = Map::new();

	for key in [
		"kind",
		"proposal_id",
		"run_id",
		"agent_id",
		"proposal_kind",
		"apply_intent",
		"review_state",
		"confidence",
		"proposed_payload_hash",
		"updated_at",
	] {
		if let Some(value) = object.get(key) {
			sanitized.insert(key.to_string(), value.clone());
		}
	}

	sanitized.insert("sanitized".to_string(), Value::Bool(true));
	sanitized.insert(
		"source_visibility".to_string(),
		Value::String("proposal_metadata_only".to_string()),
	);
	sanitized.insert(
		"omitted_fields".to_string(),
		serde_json::json!([
			"source_refs",
			"source_snapshot",
			"lineage",
			"diff",
			"unsupported_claim_flags",
			"contradiction_markers",
			"staleness_markers",
			"target_ref"
		]),
	);
	sanitized.insert(
		"nested_source_ref_count".to_string(),
		Value::Number(Number::from(nested_source_count)),
	);

	Value::Object(sanitized)
}
