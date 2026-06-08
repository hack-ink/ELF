//! Consolidation proposal contract validation.

use std::{
	error::Error,
	fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// Current consolidation contract schema identifier.
pub const CONSOLIDATION_CONTRACT_SCHEMA_V1: &str = "elf.consolidation/v1";

const FORBIDDEN_DIFF_KEYS: [&str; 7] = [
	"delete_source",
	"delete_sources",
	"source_delete",
	"source_mutation",
	"source_mutations",
	"source_note_updates",
	"overwrite_source",
];

/// Error returned by consolidation contract validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConsolidationValidationError {
	/// A required source reference list was empty.
	MissingSourceRefs,
	/// A source snapshot did not include any immutable freshness guard.
	MissingSourceSnapshot,
	/// A JSON field was not the required object shape.
	InvalidJsonObject {
		/// Name of the invalid field.
		field: &'static str,
	},
	/// A required text field was empty.
	EmptyText {
		/// Name of the invalid field.
		field: &'static str,
	},
	/// A confidence value was outside the inclusive range 0.0 to 1.0.
	InvalidConfidence,
	/// The proposal diff included a source mutation key.
	DestructiveDiff,
	/// A proposal review transition is not allowed by the lifecycle.
	InvalidReviewTransition {
		/// Current review state.
		from: ConsolidationReviewState,
		/// Requested review state.
		to: ConsolidationReviewState,
	},
	/// A run state transition is not allowed by the job lifecycle.
	InvalidRunTransition {
		/// Current run state.
		from: ConsolidationRunState,
		/// Requested run state.
		to: ConsolidationRunState,
	},
	/// A stored state string is not part of the contract.
	UnknownState {
		/// Name of the invalid field.
		field: &'static str,
	},
}
impl Display for ConsolidationValidationError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MissingSourceRefs => write!(f, "source_refs must not be empty"),
			Self::MissingSourceSnapshot =>
				write!(f, "source snapshot must include at least one freshness guard"),
			Self::InvalidJsonObject { field } => write!(f, "{field} must be a JSON object"),
			Self::EmptyText { field } => write!(f, "{field} must not be empty"),
			Self::InvalidConfidence => write!(f, "confidence must be in the range 0.0..=1.0"),
			Self::DestructiveDiff => write!(f, "proposal diff must not mutate source memory"),
			Self::InvalidReviewTransition { from, to } =>
				write!(f, "invalid proposal review transition from {from:?} to {to:?}"),
			Self::InvalidRunTransition { from, to } =>
				write!(f, "invalid consolidation run transition from {from:?} to {to:?}"),
			Self::UnknownState { field } => write!(f, "{field} is not a known state"),
		}
	}
}
impl Error for ConsolidationValidationError {}

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

/// Confidence or honesty marker severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationMarkerSeverity {
	/// Low-severity marker.
	Low,
	/// Medium-severity marker.
	Medium,
	/// High-severity marker.
	High,
}

/// One contradiction or staleness marker attached to a proposal.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationMarker {
	/// Marker severity.
	pub severity: ConsolidationMarkerSeverity,
	/// Human-readable marker text.
	pub message: String,
	/// Optional source that triggered the marker.
	pub source: Option<ConsolidationInputRef>,
}
impl ConsolidationMarker {
	/// Validates marker content and optional source evidence.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		if self.message.trim().is_empty() {
			return Err(ConsolidationValidationError::EmptyText { field: "marker.message" });
		}

		if let Some(source) = &self.source {
			source.validate()?;
		}

		Ok(())
	}
}

/// Contradiction and staleness markers attached to a proposal.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationMarkers {
	#[serde(default)]
	/// Contradiction markers that a reviewer must inspect.
	pub contradictions: Vec<ConsolidationMarker>,
	#[serde(default)]
	/// Staleness markers that a reviewer must inspect.
	pub staleness: Vec<ConsolidationMarker>,
}
impl ConsolidationMarkers {
	/// Validates all marker payloads.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		for marker in self.contradictions.iter().chain(self.staleness.iter()) {
			marker.validate()?;
		}

		Ok(())
	}
}

/// Derived-output apply intent for a reviewable proposal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationApplyIntent {
	/// Create a new derived memory note after review.
	CreateDerivedNote,
	/// Update an existing derived memory note after review.
	UpdateDerivedNote,
	/// Create a derived knowledge page after review.
	CreateDerivedKnowledgePage,
	/// Update a derived knowledge page after review.
	UpdateDerivedKnowledgePage,
	/// Create or refresh a derived graph view after review.
	CreateDerivedGraphView,
	/// Store the proposal for review without applying a downstream derived artifact.
	NoOp,
}
impl ConsolidationApplyIntent {
	/// Returns the canonical storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::CreateDerivedNote => "create_derived_note",
			Self::UpdateDerivedNote => "update_derived_note",
			Self::CreateDerivedKnowledgePage => "create_derived_knowledge_page",
			Self::UpdateDerivedKnowledgePage => "update_derived_knowledge_page",
			Self::CreateDerivedGraphView => "create_derived_graph_view",
			Self::NoOp => "no_op",
		}
	}
}

/// Review lifecycle for a consolidation proposal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationReviewState {
	/// Proposal is awaiting review.
	Proposed,
	/// Proposal has been approved for downstream derived-output application.
	Approved,
	/// Proposal was rejected by a reviewer.
	Rejected,
	/// Proposal was approved and marked applied to the derived target.
	Applied,
	/// Proposal is retained but no longer active for review.
	Archived,
}
impl ConsolidationReviewState {
	/// Returns the canonical storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Proposed => "proposed",
			Self::Approved => "approved",
			Self::Rejected => "rejected",
			Self::Applied => "applied",
			Self::Archived => "archived",
		}
	}

	/// Parses a canonical storage string.
	pub fn parse(raw: &str) -> Option<Self> {
		match raw {
			"proposed" => Some(Self::Proposed),
			"approved" => Some(Self::Approved),
			"rejected" => Some(Self::Rejected),
			"applied" => Some(Self::Applied),
			"archived" => Some(Self::Archived),
			_ => None,
		}
	}

	/// Validates a review lifecycle transition.
	pub fn validate_transition(self, to: Self) -> Result<(), ConsolidationValidationError> {
		let allowed = match self {
			Self::Proposed => matches!(to, Self::Approved | Self::Rejected | Self::Archived),
			Self::Approved => matches!(to, Self::Applied | Self::Rejected | Self::Archived),
			Self::Rejected | Self::Applied | Self::Archived => false,
		};

		if allowed {
			Ok(())
		} else {
			Err(ConsolidationValidationError::InvalidReviewTransition { from: self, to })
		}
	}
}

/// Consolidation job lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationRunState {
	/// Job has been registered but has not started.
	Pending,
	/// Job is actively generating fixture or future provider-backed proposals.
	Running,
	/// Job completed proposal generation.
	Completed,
	/// Job failed before completion.
	Failed,
	/// Job was cancelled by an operator.
	Cancelled,
}
impl ConsolidationRunState {
	/// Returns the canonical storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Pending => "pending",
			Self::Running => "running",
			Self::Completed => "completed",
			Self::Failed => "failed",
			Self::Cancelled => "cancelled",
		}
	}

	/// Parses a canonical storage string.
	pub fn parse(raw: &str) -> Option<Self> {
		match raw {
			"pending" => Some(Self::Pending),
			"running" => Some(Self::Running),
			"completed" => Some(Self::Completed),
			"failed" => Some(Self::Failed),
			"cancelled" => Some(Self::Cancelled),
			_ => None,
		}
	}

	/// Validates a job lifecycle transition.
	pub fn validate_transition(self, to: Self) -> Result<(), ConsolidationValidationError> {
		let allowed = match self {
			Self::Pending => matches!(to, Self::Running | Self::Cancelled),
			Self::Running => matches!(to, Self::Completed | Self::Failed | Self::Cancelled),
			Self::Completed | Self::Failed | Self::Cancelled => false,
		};

		if allowed {
			Ok(())
		} else {
			Err(ConsolidationValidationError::InvalidRunTransition { from: self, to })
		}
	}
}

/// Reviewable diff between prior derived output and proposed derived output.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationProposalDiff {
	/// Human-readable diff summary.
	pub summary: String,
	#[serde(default)]
	/// Previous derived output snapshot, or an empty object for creates.
	pub before: Value,
	#[serde(default)]
	/// Proposed derived output snapshot.
	pub after: Value,
}
impl ConsolidationProposalDiff {
	/// Validates diff shape and rejects source-mutation payloads.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		if self.summary.trim().is_empty() {
			return Err(ConsolidationValidationError::EmptyText { field: "diff.summary" });
		}

		validate_json_object("diff.before", &self.before)?;
		validate_json_object("diff.after", &self.after)?;

		if contains_forbidden_diff_key(&self.before) || contains_forbidden_diff_key(&self.after) {
			return Err(ConsolidationValidationError::DestructiveDiff);
		}

		Ok(())
	}
}

/// Source lineage for one consolidation proposal.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationLineage {
	/// Source references directly supporting the proposal.
	pub source_refs: Vec<ConsolidationInputRef>,
	/// Parent consolidation run, when this proposal is derived from an earlier run.
	pub parent_run_id: Option<Uuid>,
	#[serde(default)]
	/// Parent proposals used as lineage inputs.
	pub parent_proposal_ids: Vec<Uuid>,
}
impl ConsolidationLineage {
	/// Validates source lineage references.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		validate_source_refs(&self.source_refs)
	}
}

/// Full reviewable consolidation proposal contract.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationProposalContract {
	/// Proposal kind, such as `derived_note` or `knowledge_page`.
	pub proposal_kind: String,
	/// Derived-output apply intent.
	pub apply_intent: ConsolidationApplyIntent,
	/// Source references directly supporting the proposal.
	pub source_refs: Vec<ConsolidationInputRef>,
	#[serde(default)]
	/// Aggregate source snapshot metadata for reviewer inspection.
	pub source_snapshot: Value,
	/// Proposal lineage.
	pub lineage: ConsolidationLineage,
	/// Model or fixture confidence in the proposal.
	pub confidence: f32,
	/// Review markers for contradiction and staleness checks.
	pub markers: ConsolidationMarkers,
	/// Reviewable derived-output diff.
	pub diff: ConsolidationProposalDiff,
	#[serde(default)]
	/// Derived target reference, when the target already exists.
	pub target_ref: Value,
	#[serde(default)]
	/// Proposed derived output payload.
	pub proposed_payload: Value,
}
impl ConsolidationProposalContract {
	/// Validates a proposal contract before persistence.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		if self.proposal_kind.trim().is_empty() {
			return Err(ConsolidationValidationError::EmptyText { field: "proposal_kind" });
		}

		validate_source_refs(&self.source_refs)?;
		validate_json_object("source_snapshot", &self.source_snapshot)?;

		self.lineage.validate()?;

		if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
			return Err(ConsolidationValidationError::InvalidConfidence);
		}

		self.markers.validate()?;
		self.diff.validate()?;

		validate_json_object("target_ref", &self.target_ref)?;
		validate_json_object("proposed_payload", &self.proposed_payload)?;

		Ok(())
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

fn validate_json_object(
	field: &'static str,
	value: &Value,
) -> Result<(), ConsolidationValidationError> {
	if matches!(value, Value::Object(_)) {
		Ok(())
	} else {
		Err(ConsolidationValidationError::InvalidJsonObject { field })
	}
}

fn non_empty_object(value: &Value) -> bool {
	match value {
		Value::Object(map) => !map.is_empty(),
		_ => false,
	}
}

fn contains_forbidden_diff_key(value: &Value) -> bool {
	match value {
		Value::Object(map) => map.iter().any(|(key, nested)| {
			FORBIDDEN_DIFF_KEYS.contains(&key.as_str()) || contains_forbidden_diff_key(nested)
		}),
		Value::Array(items) => items.iter().any(contains_forbidden_diff_key),
		_ => false,
	}
}
