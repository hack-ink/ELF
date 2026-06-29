use crate::consolidation::{
	self, CONSOLIDATION_CONTRACT_SCHEMA_V1, ConsolidationApplyIntent, ConsolidationInputRef,
	ConsolidationMarkers, ConsolidationUnsupportedClaimFlag, ConsolidationValidationError,
	Deserialize, Serialize, Uuid, Value,
};

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

		consolidation::validate_json_object("diff.before", &self.before)?;
		consolidation::validate_json_object("diff.after", &self.after)?;

		if consolidation::contains_forbidden_diff_key(&self.before)
			|| consolidation::contains_forbidden_diff_key(&self.after)
		{
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
		consolidation::validate_source_refs(&self.source_refs)
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
	#[serde(default)]
	/// Unsupported claims that the reviewer must inspect before accepting a proposal.
	pub unsupported_claim_flags: Vec<ConsolidationUnsupportedClaimFlag>,
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

		consolidation::validate_source_refs(&self.source_refs)?;
		consolidation::validate_json_object("source_snapshot", &self.source_snapshot)?;

		self.lineage.validate()?;

		if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
			return Err(ConsolidationValidationError::InvalidConfidence);
		}

		self.markers.validate()?;

		for flag in &self.unsupported_claim_flags {
			flag.validate()?;
		}

		self.diff.validate()?;

		consolidation::validate_json_object("target_ref", &self.target_ref)?;
		consolidation::validate_json_object("proposed_payload", &self.proposed_payload)?;

		Ok(())
	}
}

/// Worker payload for materializing one consolidation run.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationJobPayload {
	/// Versioned consolidation contract schema.
	pub contract_schema: String,
	#[serde(default)]
	/// Proposals to persist for review.
	pub proposals: Vec<ConsolidationProposalContract>,
}
impl ConsolidationJobPayload {
	/// Validates the queued worker payload and all proposal contracts.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		if self.contract_schema != CONSOLIDATION_CONTRACT_SCHEMA_V1 {
			return Err(ConsolidationValidationError::InvalidContractSchema);
		}

		for proposal in &self.proposals {
			proposal.validate()?;
		}

		Ok(())
	}
}
