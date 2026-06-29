use crate::writegate::{
	WritePolicy, WritePolicyAudit, WritePolicyError, WritePolicyResult, WriteRedaction,
	WriteRedactionResult, WriteSpan,
};

#[derive(Clone, Debug)]
enum WriteOpKind {
	Exclude,
	Redact(String),
}

#[derive(Clone, Debug)]
struct WriteOp {
	span: WriteSpan,
	kind: WriteOpKind,
}

/// Applies an optional write policy to note text and returns the transformed output.
pub fn apply_write_policy(
	text: &str,
	policy: Option<&WritePolicy>,
) -> Result<WritePolicyResult, WritePolicyError> {
	let policy = match policy {
		Some(policy) => policy,
		None => {
			return Ok(WritePolicyResult {
				transformed: text.to_string(),
				audit: WritePolicyAudit::default(),
			});
		},
	};
	let mut exclusions = policy.exclusions.clone();
	let mut redactions = policy.redactions.clone();

	if exclusions.is_empty() && redactions.is_empty() {
		return Ok(WritePolicyResult {
			transformed: text.to_string(),
			audit: WritePolicyAudit::default(),
		});
	}

	exclusions.sort_by_key(|span| (span.start, span.end));
	redactions.sort_by_key(|r| match r {
		WriteRedaction::Replace { span, .. } => (span.start, span.end),
		WriteRedaction::Remove { span } => (span.start, span.end),
	});

	let mut ops = Vec::with_capacity(exclusions.len() + redactions.len());
	let mut audit = WritePolicyAudit::default();

	for span in &exclusions {
		validate_span(text, span)?;

		ops.push(WriteOp { span: *span, kind: WriteOpKind::Exclude });
		audit.exclusions.push(*span);
	}
	for redaction in &redactions {
		match redaction {
			WriteRedaction::Remove { span } => {
				validate_span(text, span)?;

				ops.push(WriteOp { span: *span, kind: WriteOpKind::Redact(String::new()) });
				audit
					.redactions
					.push(WriteRedactionResult { span: *span, replacement: String::new() });
			},

			WriteRedaction::Replace { span, replacement } => {
				validate_span(text, span)?;

				ops.push(WriteOp { span: *span, kind: WriteOpKind::Redact(replacement.clone()) });
				audit
					.redactions
					.push(WriteRedactionResult { span: *span, replacement: replacement.clone() });
			},
		}
	}

	ops.sort_by_key(|op| (op.span.start, op.span.end));

	validate_non_overlapping_ops(&ops)?;

	let mut transformed = text.to_string();

	for op in ops.iter().rev() {
		match &op.kind {
			WriteOpKind::Exclude => transformed.replace_range(op.span.start..op.span.end, ""),
			WriteOpKind::Redact(replacement) =>
				transformed.replace_range(op.span.start..op.span.end, replacement.as_str()),
		}
	}

	Ok(WritePolicyResult { transformed, audit })
}

fn validate_span(text: &str, span: &WriteSpan) -> Result<(), WritePolicyError> {
	if span.end < span.start {
		return Err(WritePolicyError::InvalidSpan);
	}
	if span.end > text.len() {
		return Err(WritePolicyError::InvalidSpan);
	}
	if !text.is_char_boundary(span.start) || !text.is_char_boundary(span.end) {
		return Err(WritePolicyError::InvalidSpan);
	}

	Ok(())
}

fn validate_non_overlapping_ops(ops: &[WriteOp]) -> Result<(), WritePolicyError> {
	let mut last_end = 0_usize;

	for op in ops {
		if op.span.start < last_end {
			return Err(WritePolicyError::OverlappingOps);
		}

		last_end = op.span.end;
	}

	Ok(())
}
