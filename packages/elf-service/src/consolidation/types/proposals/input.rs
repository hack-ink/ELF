use serde::Deserialize;
use serde_json::Value;

use crate::consolidation::types::empty_object;
use elf_domain::consolidation::{
	ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarkers,
	ConsolidationProposalContract, ConsolidationProposalDiff, ConsolidationUnsupportedClaimFlag,
};

/// Fixture proposal input for a consolidation run.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalInput {
	/// Proposal kind, such as `derived_note` or `knowledge_page`.
	pub proposal_kind: String,
	/// Derived-output apply intent.
	pub apply_intent: ConsolidationApplyIntent,
	/// Source references directly supporting the proposal.
	pub source_refs: Vec<ConsolidationInputRef>,
	#[serde(default = "empty_object")]
	/// Aggregate source snapshot metadata for reviewer inspection.
	pub source_snapshot: Value,
	/// Proposal lineage.
	pub lineage: ConsolidationLineage,
	/// Fixture confidence in the proposal.
	pub confidence: f32,
	#[serde(default)]
	/// Unsupported claims reviewers must inspect before accepting the proposal.
	pub unsupported_claim_flags: Vec<ConsolidationUnsupportedClaimFlag>,
	#[serde(default)]
	/// Review markers for contradiction and staleness checks.
	pub markers: ConsolidationMarkers,
	/// Reviewable derived-output diff.
	pub diff: ConsolidationProposalDiff,
	#[serde(default = "empty_object")]
	/// Derived target reference, when the target already exists.
	pub target_ref: Value,
	#[serde(default = "empty_object")]
	/// Proposed derived output payload.
	pub proposed_payload: Value,
}
impl ConsolidationProposalInput {
	pub(in crate::consolidation) fn into_contract(self) -> ConsolidationProposalContract {
		ConsolidationProposalContract {
			proposal_kind: self.proposal_kind,
			apply_intent: self.apply_intent,
			source_refs: self.source_refs,
			source_snapshot: self.source_snapshot,
			lineage: self.lineage,
			confidence: self.confidence,
			unsupported_claim_flags: self.unsupported_claim_flags,
			markers: self.markers,
			diff: self.diff,
			target_ref: self.target_ref,
			proposed_payload: self.proposed_payload,
		}
	}
}
