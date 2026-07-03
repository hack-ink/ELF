mod helpers;

use crate::docs::{
	ByteChunk, DOC_SOURCE_CAPTURE_SCHEMA_V1, DOC_SOURCE_SPAN_SCHEMA_V1, DocChunk, DocType,
	DocsSourceCaptureSummary, DocsSourceLifecycle, DocsSourceSpanRef, Error, Map, OffsetDateTime,
	Result, SourceCaptureSummaryInput, Uuid, Value,
};

pub(super) fn build_doc_chunk_rows(
	doc_id: Uuid,
	chunks: &[ByteChunk],
	now: OffsetDateTime,
) -> Vec<DocChunk> {
	chunks
		.iter()
		.enumerate()
		.map(|(chunk_index, chunk)| DocChunk {
			chunk_id: doc_chunk_id_for(doc_id, chunk_index as i32),
			doc_id,
			chunk_index: chunk_index as i32,
			start_offset: chunk.start_offset as i32,
			end_offset: chunk.end_offset as i32,
			chunk_text: chunk.text.clone(),
			chunk_hash: blake3::hash(chunk.text.as_bytes()).to_hex().to_string(),
			created_at: now,
		})
		.collect()
}

pub(super) fn doc_chunk_id_for(doc_id: Uuid, chunk_index: i32) -> Uuid {
	let name = format!("elf-doc-chunk/v1:{doc_id}:{chunk_index}");

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

pub(super) fn source_record_id_for(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	doc_type: DocType,
	source_ref: &Map<String, Value>,
	content_hash: &str,
) -> Uuid {
	let name = serde_json::json!([
		"elf-doc-source-record/v1",
		tenant_id,
		project_id,
		agent_id,
		scope,
		doc_type.as_str(),
		helpers::source_identity_value(source_ref, doc_type),
		content_hash,
	])
	.to_string();

	Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes())
}

pub(super) fn source_span_id(
	content_hash: &str,
	start: usize,
	end: usize,
	span_kind: &str,
) -> Uuid {
	let name = serde_json::json!(["elf-doc-source-span/v1", content_hash, start, end, span_kind,])
		.to_string();

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}

pub(super) fn build_source_capture_summary(
	input: SourceCaptureSummaryInput<'_>,
) -> Result<DocsSourceCaptureSummary> {
	let SourceCaptureSummaryInput {
		doc_id,
		source_ref,
		doc_type,
		scope,
		actor_agent_id,
		title,
		content_hash,
		raw_content_hash,
		now,
		chunks,
		write_policy_audit,
	} = input;
	let captured_at = source_ref
		.get("captured_at")
		.and_then(Value::as_str)
		.map(ToString::to_string)
		.unwrap_or(helpers::format_timestamp(now)?);
	let source_spans = chunks
		.iter()
		.map(|chunk| DocsSourceSpanRef {
			schema: DOC_SOURCE_SPAN_SCHEMA_V1.to_string(),
			span_id: source_span_id(
				content_hash,
				chunk.start_offset.max(0) as usize,
				chunk.end_offset.max(0) as usize,
				"captured",
			),
			chunk_id: Some(chunk.chunk_id),
			status: "captured".to_string(),
			reason_code: None,
			start_offset: chunk.start_offset.max(0) as usize,
			end_offset: chunk.end_offset.max(0) as usize,
			content_hash: content_hash.to_string(),
			chunk_hash: Some(chunk.chunk_hash.clone()),
		})
		.collect();
	let policy_spans = helpers::source_policy_spans(raw_content_hash, write_policy_audit);
	let captured_at_for_lifecycle = captured_at.clone();

	Ok(DocsSourceCaptureSummary {
		schema: DOC_SOURCE_CAPTURE_SCHEMA_V1.to_string(),
		source_record_id: doc_id,
		lifecycle: DocsSourceLifecycle {
			schema: "elf.source_lifecycle/v1".to_string(),
			status: "active".to_string(),
			freshness: "current".to_string(),
			actor_agent_id: actor_agent_id.to_string(),
			ts: captured_at_for_lifecycle,
			reason_code: "SOURCE_CAPTURED".to_string(),
			deleted_at: None,
			tombstone_ref: None,
		},
		origin: helpers::source_origin(source_ref, doc_type),
		captured_at,
		content_hash: content_hash.to_string(),
		visibility_scope: scope.to_string(),
		title: title.map(ToString::to_string),
		source_type: helpers::source_type(source_ref, doc_type),
		source_spans,
		policy_spans,
	})
}

pub(super) fn normalize_source_ref_for_capture(
	source_ref: Value,
	source_capture: &DocsSourceCaptureSummary,
) -> Result<Value> {
	let mut source_ref = source_ref.as_object().cloned().ok_or_else(|| Error::InvalidRequest {
		message: "source_ref must be a JSON object.".to_string(),
	})?;

	source_ref.insert(
		"source_record_id".to_string(),
		Value::String(source_capture.source_record_id.to_string()),
	);
	source_ref.insert(
		"lifecycle".to_string(),
		serde_json::to_value(&source_capture.lifecycle).map_err(|_| Error::InvalidRequest {
			message: "Failed to serialize source lifecycle.".to_string(),
		})?,
	);
	source_ref.insert("origin".to_string(), Value::String(source_capture.origin.clone()));
	source_ref.insert("captured_at".to_string(), Value::String(source_capture.captured_at.clone()));
	source_ref
		.insert("content_hash".to_string(), Value::String(source_capture.content_hash.clone()));
	source_ref.insert(
		"visibility_scope".to_string(),
		Value::String(source_capture.visibility_scope.clone()),
	);

	if let Some(title) = source_capture.title.as_ref() {
		source_ref.entry("title".to_string()).or_insert_with(|| Value::String(title.clone()));
	}

	source_ref.insert("source_type".to_string(), Value::String(source_capture.source_type.clone()));
	source_ref.insert(
		"source_spans".to_string(),
		helpers::source_spans_to_value(&source_capture.source_spans)?,
	);

	if !source_capture.policy_spans.is_empty() {
		source_ref.insert(
			"policy_spans".to_string(),
			helpers::source_spans_to_value(&source_capture.policy_spans)?,
		);
	}

	Ok(Value::Object(source_ref))
}

pub(super) fn source_ref_with_deleted_lifecycle(
	source_ref: &Value,
	actor_agent_id: &str,
	now: OffsetDateTime,
) -> Result<Value> {
	let ts = helpers::format_timestamp(now)?;
	let tombstone_ref = serde_json::json!({
		"schema": "elf.source_tombstone/v1",
		"reason_code": "SOURCE_LIBRARY_DELETE",
		"actor_agent_id": actor_agent_id,
		"ts": ts,
	});
	let lifecycle = DocsSourceLifecycle {
		schema: "elf.source_lifecycle/v1".to_string(),
		status: "deleted".to_string(),
		freshness: "tombstoned".to_string(),
		actor_agent_id: actor_agent_id.to_string(),
		ts: ts.clone(),
		reason_code: "SOURCE_LIBRARY_DELETE".to_string(),
		deleted_at: Some(ts),
		tombstone_ref: Some(tombstone_ref),
	};
	let mut source_ref = source_ref.as_object().cloned().ok_or_else(|| Error::InvalidRequest {
		message: "source_ref must be a JSON object.".to_string(),
	})?;

	source_ref.insert(
		"lifecycle".to_string(),
		serde_json::to_value(lifecycle).map_err(|_| Error::InvalidRequest {
			message: "Failed to serialize source lifecycle.".to_string(),
		})?,
	);

	Ok(Value::Object(source_ref))
}
