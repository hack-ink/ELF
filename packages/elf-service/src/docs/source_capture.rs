use super::*;

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
		source_identity_value(source_ref, doc_type),
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
		.unwrap_or(format_timestamp(now)?);
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
	let policy_spans = source_policy_spans(raw_content_hash, write_policy_audit);

	Ok(DocsSourceCaptureSummary {
		schema: DOC_SOURCE_CAPTURE_SCHEMA_V1.to_string(),
		source_record_id: doc_id,
		origin: source_origin(source_ref, doc_type),
		captured_at,
		content_hash: content_hash.to_string(),
		visibility_scope: scope.to_string(),
		title: title.map(ToString::to_string),
		source_type: source_type(source_ref, doc_type),
		source_spans,
		policy_spans,
	})
}

pub(super) fn source_policy_spans(
	raw_content_hash: &str,
	audit: Option<&WritePolicyAudit>,
) -> Vec<DocsSourceSpanRef> {
	let Some(audit) = audit else {
		return Vec::new();
	};
	let mut spans = Vec::with_capacity(audit.exclusions.len() + audit.redactions.len());

	for span in &audit.exclusions {
		spans.push(policy_span_ref(
			raw_content_hash,
			span.start,
			span.end,
			"excluded",
			"WRITE_POLICY_EXCLUSION",
		));
	}
	for redaction in &audit.redactions {
		spans.push(policy_span_ref(
			raw_content_hash,
			redaction.span.start,
			redaction.span.end,
			"redacted",
			"WRITE_POLICY_REDACTION",
		));
	}

	spans
}

pub(super) fn policy_span_ref(
	content_hash: &str,
	start: usize,
	end: usize,
	status: &str,
	reason_code: &str,
) -> DocsSourceSpanRef {
	DocsSourceSpanRef {
		schema: DOC_SOURCE_SPAN_SCHEMA_V1.to_string(),
		span_id: source_span_id(content_hash, start, end, reason_code),
		chunk_id: None,
		status: status.to_string(),
		reason_code: Some(reason_code.to_string()),
		start_offset: start,
		end_offset: end,
		content_hash: content_hash.to_string(),
		chunk_hash: None,
	}
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
	source_ref
		.insert("source_spans".to_string(), source_spans_to_value(&source_capture.source_spans)?);

	if !source_capture.policy_spans.is_empty() {
		source_ref.insert(
			"policy_spans".to_string(),
			source_spans_to_value(&source_capture.policy_spans)?,
		);
	}

	Ok(Value::Object(source_ref))
}

pub(super) fn source_spans_to_value(spans: &[DocsSourceSpanRef]) -> Result<Value> {
	serde_json::to_value(spans).map_err(|err| Error::InvalidRequest {
		message: format!("failed to encode source span metadata: {err}"),
	})
}

pub(super) fn source_type(source_ref: &Map<String, Value>, doc_type: DocType) -> String {
	source_ref
		.get("source_kind")
		.and_then(Value::as_str)
		.filter(|value| !value.trim().is_empty())
		.unwrap_or_else(|| doc_type.as_str())
		.to_string()
}

pub(super) fn source_origin(source_ref: &Map<String, Value>, doc_type: DocType) -> String {
	if let Some(origin) = source_ref_string(source_ref, "canonical_uri")
		.or_else(|| source_ref_string(source_ref, "url"))
		.or_else(|| source_ref_string(source_ref, "uri"))
	{
		return origin.to_string();
	}

	match doc_type {
		DocType::Chat => source_ref_string(source_ref, "message_id")
			.map(|message_id| {
				format!(
					"thread:{}#{}",
					source_ref_string(source_ref, "thread_id").unwrap_or("unknown"),
					message_id
				)
			})
			.unwrap_or_else(|| {
				format!(
					"thread:{}",
					source_ref_string(source_ref, "thread_id").unwrap_or("unknown")
				)
			}),
		DocType::Search => source_ref_string(source_ref, "domain")
			.map(|domain| format!("search:{domain}"))
			.unwrap_or_else(|| "search:unknown".to_string()),
		DocType::Dev => dev_origin(source_ref),
		DocType::Knowledge => source_ref_string(source_ref, "ts")
			.map(|ts| format!("knowledge:{ts}"))
			.unwrap_or_else(|| "knowledge:unknown".to_string()),
	}
}

pub(super) fn dev_origin(source_ref: &Map<String, Value>) -> String {
	let repo = source_ref_string(source_ref, "repo").unwrap_or("unknown");
	let path = source_ref_string(source_ref, "path").unwrap_or("");
	let revision = source_ref_string(source_ref, "commit_sha")
		.map(|commit| format!("@{commit}"))
		.or_else(|| source_ref_i64(source_ref, "pr_number").map(|pr| format!("#pr-{pr}")))
		.or_else(|| {
			source_ref_i64(source_ref, "issue_number").map(|issue| format!("#issue-{issue}"))
		})
		.unwrap_or_default();

	if path.is_empty() {
		format!("repo:{repo}{revision}")
	} else {
		format!("repo:{repo}/{path}{revision}")
	}
}

pub(super) fn source_identity_value(source_ref: &Map<String, Value>, doc_type: DocType) -> Value {
	if let Some(canonical_uri) = source_ref_string(source_ref, "canonical_uri") {
		return serde_json::json!(["canonical_uri", canonical_uri]);
	}

	match doc_type {
		DocType::Chat => serde_json::json!([
			"chat",
			source_ref_string(source_ref, "thread_id"),
			source_ref_string(source_ref, "message_id"),
			source_ref_string(source_ref, "role"),
			source_ref_string(source_ref, "ts"),
		]),
		DocType::Search => serde_json::json!([
			"search",
			source_ref_string(source_ref, "url"),
			source_ref_string(source_ref, "domain"),
			source_ref_string(source_ref, "query"),
			source_ref_string(source_ref, "ts"),
		]),
		DocType::Dev => serde_json::json!([
			"dev",
			source_ref_string(source_ref, "repo"),
			source_ref_string(source_ref, "path"),
			source_ref_string(source_ref, "commit_sha"),
			source_ref_i64(source_ref, "pr_number"),
			source_ref_i64(source_ref, "issue_number"),
		]),
		DocType::Knowledge => serde_json::json!([
			"knowledge",
			source_ref_string(source_ref, "uri"),
			source_ref_string(source_ref, "ts"),
		]),
	}
}

pub(super) fn source_ref_string<'a>(
	source_ref: &'a Map<String, Value>,
	key: &str,
) -> Option<&'a str> {
	source_ref.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

pub(super) fn source_ref_i64(source_ref: &Map<String, Value>, key: &str) -> Option<i64> {
	source_ref.get(key).and_then(Value::as_i64)
}

pub(super) fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	ts.format(&Rfc3339).map_err(|err| Error::InvalidRequest {
		message: format!("failed to format RFC3339 timestamp: {err}"),
	})
}
