use crate::docs::{
	DOC_SOURCE_SPAN_SCHEMA_V1, DocType, DocsSourceSpanRef, Error, Map, OffsetDateTime, Result,
	Rfc3339, Value, WritePolicyAudit, source_capture,
};

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

pub(super) fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	ts.format(&Rfc3339).map_err(|err| Error::InvalidRequest {
		message: format!("failed to format RFC3339 timestamp: {err}"),
	})
}

fn policy_span_ref(
	content_hash: &str,
	start: usize,
	end: usize,
	status: &str,
	reason_code: &str,
) -> DocsSourceSpanRef {
	DocsSourceSpanRef {
		schema: DOC_SOURCE_SPAN_SCHEMA_V1.to_string(),
		span_id: source_capture::source_span_id(content_hash, start, end, reason_code),
		chunk_id: None,
		status: status.to_string(),
		reason_code: Some(reason_code.to_string()),
		start_offset: start,
		end_offset: end,
		content_hash: content_hash.to_string(),
		chunk_hash: None,
	}
}

fn dev_origin(source_ref: &Map<String, Value>) -> String {
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

fn source_ref_string<'a>(source_ref: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
	source_ref.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn source_ref_i64(source_ref: &Map<String, Value>, key: &str) -> Option<i64> {
	source_ref.get(key).and_then(Value::as_i64)
}

#[cfg(test)]
mod tests {
	use crate::docs::{DocType, Map, Value, source_capture::helpers};
	use elf_domain::writegate::{WritePolicyAudit, WriteRedactionResult, WriteSpan};

	fn source_ref(value: Value) -> Map<String, Value> {
		value.as_object().expect("source_ref should be an object").clone()
	}

	#[test]
	fn source_origin_prefers_canonical_uri_and_preserves_fallbacks() {
		let canonical = source_ref(serde_json::json!({
			"canonical_uri": "https://example.com/canonical",
			"url": "https://example.com/url",
			"uri": "file:///tmp/source.txt"
		}));
		let chat = source_ref(serde_json::json!({
			"thread_id": "thread-a",
			"message_id": "message-b"
		}));
		let dev = source_ref(serde_json::json!({
			"repo": "hack-ink/ELF",
			"path": "packages/elf-service/src/docs.rs",
			"pr_number": 278
		}));
		let knowledge = source_ref(serde_json::json!({
			"ts": "2026-02-25T12:00:00Z"
		}));

		assert_eq!(
			helpers::source_origin(&canonical, DocType::Knowledge),
			"https://example.com/canonical"
		);
		assert_eq!(helpers::source_origin(&chat, DocType::Chat), "thread:thread-a#message-b");
		assert_eq!(
			helpers::source_origin(&dev, DocType::Dev),
			"repo:hack-ink/ELF/packages/elf-service/src/docs.rs#pr-278"
		);
		assert_eq!(
			helpers::source_origin(&knowledge, DocType::Knowledge),
			"knowledge:2026-02-25T12:00:00Z"
		);
	}

	#[test]
	fn source_identity_value_prefers_canonical_uri_and_preserves_type_shape() {
		let canonical = source_ref(serde_json::json!({
			"canonical_uri": "https://example.com/canonical",
			"uri": "file:///tmp/source.txt"
		}));
		let dev = source_ref(serde_json::json!({
			"repo": "hack-ink/ELF",
			"path": "packages/elf-service/src/docs.rs",
			"commit_sha": "abc123",
			"pr_number": 278
		}));

		assert_eq!(
			helpers::source_identity_value(&canonical, DocType::Knowledge),
			serde_json::json!(["canonical_uri", "https://example.com/canonical"])
		);
		assert_eq!(
			helpers::source_identity_value(&dev, DocType::Dev),
			serde_json::json!([
				"dev",
				"hack-ink/ELF",
				"packages/elf-service/src/docs.rs",
				"abc123",
				278,
				null
			])
		);
	}

	#[test]
	fn source_policy_spans_preserve_write_policy_order_and_reasons() {
		let audit = WritePolicyAudit {
			exclusions: vec![WriteSpan { start: 4, end: 10 }],
			redactions: vec![WriteRedactionResult {
				span: WriteSpan { start: 16, end: 22 },
				replacement: "[redacted]".to_string(),
			}],
		};
		let spans = helpers::source_policy_spans("raw-content-hash", Some(&audit));

		assert_eq!(spans.len(), 2);
		assert_eq!(spans[0].status, "excluded");
		assert_eq!(spans[0].reason_code.as_deref(), Some("WRITE_POLICY_EXCLUSION"));
		assert_eq!(spans[0].start_offset, 4);
		assert_eq!(spans[0].end_offset, 10);
		assert_eq!(spans[1].status, "redacted");
		assert_eq!(spans[1].reason_code.as_deref(), Some("WRITE_POLICY_REDACTION"));
		assert_eq!(spans[1].start_offset, 16);
		assert_eq!(spans[1].end_offset, 22);
	}
}
