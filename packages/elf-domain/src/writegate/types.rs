use crate::writegate::{Deserialize, Serialize};

/// Reasons a note can be rejected by the write gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectCode {
	/// The note text failed the English gate.
	RejectNonEnglish,
	/// The note text exceeded the configured length limit.
	RejectTooLong,
	/// The note text appears to contain secret material.
	RejectSecret,
	/// The note type is not one of the allowed values.
	RejectInvalidType,
	/// The note scope is not allowed or not writable.
	RejectScopeDenied,
	/// The note text is empty after trimming.
	RejectEmpty,
}

/// One write-policy redaction operation.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WriteRedaction {
	/// Replaces the target span with a literal string.
	Replace {
		/// Span to replace before persistence.
		span: WriteSpan,
		/// Literal replacement text to insert for the span.
		replacement: String,
	},
	/// Removes the target span entirely.
	Remove {
		/// Span to remove before persistence.
		span: WriteSpan,
	},
}

/// Errors returned while validating write-policy spans.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WritePolicyError {
	/// A span was out of bounds or not aligned to char boundaries.
	InvalidSpan,
	/// Two exclusions/redactions overlapped.
	OverlappingOps,
}

/// Half-open byte span within input text.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WriteSpan {
	/// Inclusive start byte offset.
	pub start: usize,
	/// Exclusive end byte offset.
	pub end: usize,
}

/// Optional write-policy transform applied before note ingestion.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WritePolicy {
	/// Spans that should be removed before persistence.
	#[serde(default)]
	pub exclusions: Vec<WriteSpan>,
	/// Redactions that should be applied before persistence.
	#[serde(default)]
	pub redactions: Vec<WriteRedaction>,
}

/// Result of applying a write policy to one note body.
#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct WritePolicyResult {
	/// Transformed note text after exclusions and redactions.
	pub transformed: String,
	/// Audit data describing which operations were applied.
	pub audit: WritePolicyAudit,
}

/// Audit payload emitted when a write policy is applied.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WritePolicyAudit {
	/// Exclusion spans that were applied.
	pub exclusions: Vec<WriteSpan>,
	/// Redactions that were applied.
	pub redactions: Vec<WriteRedactionResult>,
}

/// One redaction entry in write-policy audit output.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WriteRedactionResult {
	/// Span that was removed or replaced.
	pub span: WriteSpan,
	/// Replacement text that was applied.
	pub replacement: String,
}

/// Normalized note input passed through `writegate`.
pub struct NoteInput {
	/// Requested note type.
	pub note_type: String,
	/// Requested write scope.
	pub scope: String,
	/// Note text after request decoding.
	pub text: String,
}
