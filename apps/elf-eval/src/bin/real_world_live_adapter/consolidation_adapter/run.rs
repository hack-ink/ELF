use crate::{
	ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
	ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
	ConsolidationProposalInput, ConsolidationUnsupportedClaimFlag, CorpusText, IngestedCorpus,
	LiveConsolidationFixture, LiveConsolidationProposal, LoadedJob, PreparedConsolidationRun,
	Result, consolidation_adapter::refs, eyre, serde_json,
};

pub(in crate::consolidation_adapter) fn prepare_consolidation_run(
	loaded: &LoadedJob,
	adapter_id: &str,
	ingested: &IngestedCorpus,
	fixture: &LiveConsolidationFixture,
	corpus: &[CorpusText],
) -> Result<PreparedConsolidationRun> {
	let mut input_refs = Vec::new();
	let mut proposals = Vec::new();

	for proposal in &fixture.proposals {
		let source_refs = refs::consolidation_input_refs(
			loaded,
			adapter_id,
			proposal.source_refs.as_slice(),
			ingested,
			corpus,
		)?;

		for source_ref in &source_refs {
			refs::push_unique_input_ref(&mut input_refs, source_ref.clone());
		}

		proposals.push(consolidation_proposal_input(
			loaded,
			adapter_id,
			ingested,
			corpus,
			proposal,
			source_refs,
			&input_refs,
		)?);
	}

	if proposals.is_empty() {
		return Err(eyre::eyre!("{} has no consolidation proposals.", loaded.job.job_id));
	}

	Ok(PreparedConsolidationRun { input_refs, proposals })
}

fn consolidation_proposal_input(
	loaded: &LoadedJob,
	adapter_id: &str,
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
	proposal: &LiveConsolidationProposal,
	source_refs: Vec<ConsolidationInputRef>,
	input_refs: &[ConsolidationInputRef],
) -> Result<ConsolidationProposalInput> {
	let unsupported_claim_flags =
		consolidation_unsupported_claim_flags(loaded, adapter_id, proposal, ingested, corpus)?;
	let diff = consolidation_diff(proposal.diff.clone())?;
	let proposed_payload = object_or_empty(diff.after.clone());
	let lineage = ConsolidationLineage {
		source_refs: source_refs.clone(),
		parent_run_id: None,
		parent_proposal_ids: Vec::new(),
	};

	Ok(ConsolidationProposalInput {
		proposal_kind: proposal.proposal_kind.clone(),
		apply_intent: consolidation_apply_intent(proposal.actual_review_action.as_str()),
		source_refs,
		source_snapshot: serde_json::json!({
			"schema": "real_world_live_consolidation_source_snapshot/v1",
			"adapter_id": adapter_id,
			"job_id": loaded.job.job_id,
			"proposal_id": proposal.proposal_id
		}),
		lineage,
		confidence: proposal.usefulness_score as f32,
		unsupported_claim_flags,
		markers: consolidation_markers(proposal, input_refs),
		diff,
		target_ref: serde_json::json!({
			"schema": "real_world_live_consolidation_target/v1",
			"proposal_id": proposal.proposal_id
		}),
		proposed_payload,
	})
}

fn consolidation_unsupported_claim_flags(
	loaded: &LoadedJob,
	adapter_id: &str,
	proposal: &LiveConsolidationProposal,
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
) -> Result<Vec<ConsolidationUnsupportedClaimFlag>> {
	proposal
		.unsupported_claim_flags
		.iter()
		.map(|flag| {
			let source = flag
				.source_ref
				.as_deref()
				.map(|source_ref| {
					refs::consolidation_input_refs(
						loaded,
						adapter_id,
						&[source_ref.to_string()],
						ingested,
						corpus,
					)
					.and_then(|refs| {
						refs.into_iter().next().ok_or_else(|| {
							eyre::eyre!(
								"Unsupported claim source {} did not map to a live source.",
								source_ref
							)
						})
					})
				})
				.transpose()?;

			Ok(ConsolidationUnsupportedClaimFlag {
				claim_id: flag.claim_id.clone(),
				message: flag.message.clone(),
				source,
			})
		})
		.collect()
}

fn consolidation_diff(value: serde_json::Value) -> Result<ConsolidationProposalDiff> {
	let summary = value
		.get("summary")
		.and_then(serde_json::Value::as_str)
		.unwrap_or("Live consolidation proposal.")
		.to_string();

	Ok(ConsolidationProposalDiff {
		summary,
		before: object_or_empty(value.get("before").cloned().unwrap_or(serde_json::Value::Null)),
		after: object_or_empty(value.get("after").cloned().unwrap_or(serde_json::Value::Null)),
	})
}

fn object_or_empty(value: serde_json::Value) -> serde_json::Value {
	if matches!(value, serde_json::Value::Object(_)) { value } else { serde_json::json!({}) }
}

fn consolidation_apply_intent(action: &str) -> ConsolidationApplyIntent {
	if action == "apply" {
		ConsolidationApplyIntent::CreateDerivedNote
	} else {
		ConsolidationApplyIntent::NoOp
	}
}

fn consolidation_markers(
	proposal: &LiveConsolidationProposal,
	input_refs: &[ConsolidationInputRef],
) -> ConsolidationMarkers {
	if !proposal.proposal_kind.contains("contradiction") {
		return ConsolidationMarkers::default();
	}

	let marker = ConsolidationMarker {
		severity: ConsolidationMarkerSeverity::High,
		message:
			"Live adapter materialized a contradiction-oriented proposal for reviewer inspection."
				.to_string(),
		source: input_refs.first().cloned(),
	};

	ConsolidationMarkers { contradictions: vec![marker], staleness: Vec::new() }
}
