use crate::consolidation::{ConsolidationValidationError, Deserialize, Serialize};

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

/// Reviewer action requested for a consolidation proposal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationReviewAction {
	/// Approve a proposal for later application.
	Approve,
	/// Apply an approved proposal to a derived target.
	Apply,
	/// Discard a proposal as rejected.
	Discard,
	/// Defer a proposal by archiving it for later audit.
	Defer,
}
impl ConsolidationReviewAction {
	/// Returns the canonical storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Approve => "approve",
			Self::Apply => "apply",
			Self::Discard => "discard",
			Self::Defer => "defer",
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
