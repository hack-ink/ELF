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
		from: super::lifecycle::ConsolidationReviewState,
		/// Requested review state.
		to: super::lifecycle::ConsolidationReviewState,
	},
	/// A run state transition is not allowed by the job lifecycle.
	InvalidRunTransition {
		/// Current run state.
		from: super::lifecycle::ConsolidationRunState,
		/// Requested run state.
		to: super::lifecycle::ConsolidationRunState,
	},
	/// A stored state string is not part of the contract.
	UnknownState {
		/// Name of the invalid field.
		field: &'static str,
	},
	/// The queued contract schema did not match the consolidation v1 contract.
	InvalidContractSchema,
}
impl std::fmt::Display for ConsolidationValidationError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MissingSourceRefs => write!(f, "source_refs must not be empty"),
			Self::MissingSourceSnapshot => {
				write!(f, "source snapshot must include at least one freshness guard")
			},
			Self::InvalidJsonObject { field } => write!(f, "{field} must be a JSON object"),
			Self::EmptyText { field } => write!(f, "{field} must not be empty"),
			Self::InvalidConfidence => write!(f, "confidence must be in the range 0.0..=1.0"),
			Self::DestructiveDiff => write!(f, "proposal diff must not mutate source memory"),
			Self::InvalidReviewTransition { from, to } => {
				write!(f, "invalid proposal review transition from {from:?} to {to:?}")
			},
			Self::InvalidRunTransition { from, to } => {
				write!(f, "invalid consolidation run transition from {from:?} to {to:?}")
			},
			Self::UnknownState { field } => write!(f, "{field} is not a known state"),
			Self::InvalidContractSchema => {
				write!(f, "contract_schema must be elf.consolidation/v1")
			},
		}
	}
}
impl std::error::Error for ConsolidationValidationError {}
