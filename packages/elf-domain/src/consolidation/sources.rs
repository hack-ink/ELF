use super::*;

/// Source artifact kind accepted by consolidation input references.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationSourceKind {
	/// Memory note evidence.
	Note,
	/// Event ingestion source.
	Event,
	/// Search trace source.
	Trace,
	/// Search trace item source.
	TraceItem,
	/// Document extension source.
	Doc,
	/// Document chunk source.
	DocChunk,
}
impl ConsolidationSourceKind {
	/// Returns the canonical storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Note => "note",
			Self::Event => "event",
			Self::Trace => "trace",
			Self::TraceItem => "trace_item",
			Self::Doc => "doc",
			Self::DocChunk => "doc_chunk",
		}
	}
}

/// Immutable source snapshot guard captured before a proposal is stored.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationSourceSnapshot {
	/// Source lifecycle status observed by the consolidation run.
	pub status: Option<String>,
	/// Source last-update timestamp observed by the consolidation run.
	pub updated_at: Option<OffsetDateTime>,
	/// Source content or payload hash, when available.
	pub content_hash: Option<String>,
	/// Source embedding version, when relevant.
	pub embedding_version: Option<String>,
	/// Trace schema or trace version, when the source is a trace.
	pub trace_version: Option<i32>,
	#[serde(default)]
	/// Opaque source reference copied from the authoritative source.
	pub source_ref: Value,
	#[serde(default)]
	/// Additional snapshot metadata used for replay or review.
	pub metadata: Value,
}
impl ConsolidationSourceSnapshot {
	/// Validates snapshot shape and immutable freshness guards.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		validate_json_object("source_ref", &self.source_ref)?;
		validate_json_object("metadata", &self.metadata)?;

		let has_hash = self.content_hash.as_ref().is_some_and(|hash| !hash.trim().is_empty());
		let has_embedding =
			self.embedding_version.as_ref().is_some_and(|version| !version.trim().is_empty());
		let has_status = self.status.as_ref().is_some_and(|status| !status.trim().is_empty());
		let has_source_ref = non_empty_object(&self.source_ref);
		let has_metadata = non_empty_object(&self.metadata);
		let has_guard = self.updated_at.is_some()
			|| self.trace_version.is_some()
			|| has_hash
			|| has_embedding
			|| has_status
			|| has_source_ref
			|| has_metadata;

		if has_guard { Ok(()) } else { Err(ConsolidationValidationError::MissingSourceSnapshot) }
	}
}

/// Stable pointer to one immutable consolidation input.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationInputRef {
	/// Kind of source artifact being referenced.
	pub kind: ConsolidationSourceKind,
	/// Identifier of the source artifact.
	pub id: Uuid,
	/// Snapshot metadata captured before proposal generation.
	pub snapshot: ConsolidationSourceSnapshot,
}
impl ConsolidationInputRef {
	/// Validates the input reference and its snapshot guard.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		self.snapshot.validate()
	}
}

/// Validates a source reference list.
pub fn validate_source_refs(
	source_refs: &[ConsolidationInputRef],
) -> Result<(), ConsolidationValidationError> {
	if source_refs.is_empty() {
		return Err(ConsolidationValidationError::MissingSourceRefs);
	}

	for source_ref in source_refs {
		source_ref.validate()?;
	}

	Ok(())
}
