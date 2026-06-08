#![allow(unused_crate_dependencies)]

//! Integration tests for consolidation proposal contract validation.

use time::OffsetDateTime;
use uuid::Uuid;

use elf_domain::consolidation::{
	ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarkers,
	ConsolidationProposalContract, ConsolidationProposalDiff, ConsolidationReviewState,
	ConsolidationRunState, ConsolidationSourceKind, ConsolidationSourceSnapshot,
	ConsolidationValidationError,
};

#[test]
fn proposal_contract_accepts_reviewable_derived_output() {
	let source = source_ref();
	let proposal = proposal_contract(source);

	assert!(proposal.validate().is_ok());
}

#[test]
fn source_refs_require_immutable_snapshot_guards() {
	let mut source = source_ref();

	source.snapshot = ConsolidationSourceSnapshot {
		status: None,
		updated_at: None,
		content_hash: None,
		embedding_version: None,
		trace_version: None,
		source_ref: serde_json::json!({}),
		metadata: serde_json::json!({}),
	};

	assert_eq!(source.validate(), Err(ConsolidationValidationError::MissingSourceSnapshot));
}

#[test]
fn proposal_contract_requires_lineage_source_refs() {
	let source = source_ref();
	let mut proposal = proposal_contract(source);

	proposal.lineage.source_refs = Vec::new();

	assert_eq!(proposal.validate(), Err(ConsolidationValidationError::MissingSourceRefs));
}

#[test]
fn proposal_contract_rejects_destructive_diff_payloads() {
	let source = source_ref();
	let mut proposal = proposal_contract(source);

	proposal.diff.after = serde_json::json!({
		"summary": "Replace stale source facts.",
		"source_mutations": [
			{ "kind": "note", "op": "delete" }
		]
	});

	assert_eq!(proposal.validate(), Err(ConsolidationValidationError::DestructiveDiff));
}

#[test]
fn destructive_apply_intents_are_not_part_of_the_contract() {
	let parsed =
		serde_json::from_value::<ConsolidationApplyIntent>(serde_json::json!("delete_source_note"));

	assert!(parsed.is_err());
}

#[test]
fn proposal_lifecycle_requires_approval_before_apply() {
	assert!(
		ConsolidationReviewState::Proposed
			.validate_transition(ConsolidationReviewState::Applied)
			.is_err()
	);
	assert!(
		ConsolidationReviewState::Proposed
			.validate_transition(ConsolidationReviewState::Approved)
			.is_ok()
	);
	assert!(
		ConsolidationReviewState::Approved
			.validate_transition(ConsolidationReviewState::Applied)
			.is_ok()
	);
	assert!(
		ConsolidationReviewState::Applied
			.validate_transition(ConsolidationReviewState::Rejected)
			.is_err()
	);
}

#[test]
fn run_lifecycle_rejects_skipping_generation_state() {
	assert!(
		ConsolidationRunState::Pending
			.validate_transition(ConsolidationRunState::Completed)
			.is_err()
	);
	assert!(
		ConsolidationRunState::Pending.validate_transition(ConsolidationRunState::Running).is_ok()
	);
	assert!(
		ConsolidationRunState::Running
			.validate_transition(ConsolidationRunState::Completed)
			.is_ok()
	);
}

fn proposal_contract(source: ConsolidationInputRef) -> ConsolidationProposalContract {
	let lineage = ConsolidationLineage {
		source_refs: vec![source.clone()],
		parent_run_id: None,
		parent_proposal_ids: Vec::new(),
	};

	ConsolidationProposalContract {
		proposal_kind: "derived_note".to_string(),
		apply_intent: ConsolidationApplyIntent::CreateDerivedNote,
		source_refs: vec![source],
		source_snapshot: serde_json::json!({ "window": "fixture" }),
		lineage,
		confidence: 0.85,
		markers: ConsolidationMarkers::default(),
		diff: ConsolidationProposalDiff {
			summary: "Create one derived note from stable evidence.".to_string(),
			before: serde_json::json!({}),
			after: serde_json::json!({ "text": "Fact: The project keeps consolidation output reviewable." }),
		},
		target_ref: serde_json::json!({}),
		proposed_payload: serde_json::json!({
			"type": "fact",
			"text": "Fact: The project keeps consolidation output reviewable."
		}),
	}
}

fn source_ref() -> ConsolidationInputRef {
	ConsolidationInputRef {
		kind: ConsolidationSourceKind::Note,
		id: Uuid::parse_str("11111111-1111-1111-1111-111111111111")
			.expect("test UUID must be valid"),
		snapshot: ConsolidationSourceSnapshot {
			status: Some("active".to_string()),
			updated_at: Some(OffsetDateTime::UNIX_EPOCH),
			content_hash: Some("blake3:fixture".to_string()),
			embedding_version: Some("fixture:model:4".to_string()),
			trace_version: None,
			source_ref: serde_json::json!({ "schema": "source_ref/v1", "resolver": "fixture" }),
			metadata: serde_json::json!({}),
		},
	}
}
