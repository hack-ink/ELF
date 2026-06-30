use crate::docs::{
	DOC_SOURCE_SPAN_SCHEMA_V1, DocsSourceSpanRef, Error, Result, Value, WritePolicyAudit,
	source_capture,
};

pub(in crate::docs::source_capture) fn source_policy_spans(
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

pub(in crate::docs::source_capture) fn source_spans_to_value(
	spans: &[DocsSourceSpanRef],
) -> Result<Value> {
	serde_json::to_value(spans).map_err(|err| Error::InvalidRequest {
		message: format!("failed to encode source span metadata: {err}"),
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
