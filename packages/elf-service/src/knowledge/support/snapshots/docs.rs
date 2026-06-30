use crate::knowledge::support::{
	self, KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeSourceKind, SourceSnapshot,
	serde_json,
};

pub(in crate::knowledge) fn doc_source_snapshot(row: KnowledgeDocSource) -> SourceSnapshot {
	let title = row.title.clone().unwrap_or_else(|| "Untitled source document".to_string());
	let excerpt =
		support::truncate_chars(support::normalize_whitespace(row.content.as_str()).as_str(), 240);
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
	let excerpt = support::truncate_chars(
		support::normalize_whitespace(row.chunk_text.as_str()).as_str(),
		240,
	);
	let span_id = support::source_span_id(
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
