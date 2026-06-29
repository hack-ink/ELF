use super::*;

pub(super) fn live_consolidation_fixture(
	loaded: &LoadedJob,
) -> color_eyre::Result<LiveConsolidationFixture> {
	let value =
		loaded.value.pointer("/corpus/adapter_response/consolidation").cloned().ok_or_else(
			|| {
				eyre::eyre!(
					"{} does not contain adapter_response.consolidation.",
					loaded.path.display()
				)
			},
		)?;

	serde_json::from_value(value).map_err(|err| {
		eyre::eyre!("Failed to parse consolidation fixture {}: {err}", loaded.path.display())
	})
}

pub(super) fn prepare_consolidation_run(
	loaded: &LoadedJob,
	adapter_id: &str,
	ingested: &IngestedCorpus,
	fixture: &LiveConsolidationFixture,
	corpus: &[CorpusText],
) -> color_eyre::Result<PreparedConsolidationRun> {
	let mut input_refs = Vec::new();
	let mut proposals = Vec::new();

	for proposal in &fixture.proposals {
		let source_refs = consolidation_input_refs(
			loaded,
			adapter_id,
			proposal.source_refs.as_slice(),
			ingested,
			corpus,
		)?;

		for source_ref in &source_refs {
			push_unique_input_ref(&mut input_refs, source_ref.clone());
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
) -> color_eyre::Result<ConsolidationProposalInput> {
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

pub(super) fn validate_reviewed_consolidation_count(
	loaded: &LoadedJob,
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> color_eyre::Result<()> {
	if reviewed.len() == fixture.proposals.len() {
		return Ok(());
	}

	Err(eyre::eyre!(
		"ELF consolidation materialized {} proposals for {} fixture proposals in {}.",
		reviewed.len(),
		fixture.proposals.len(),
		loaded.job.job_id
	))
}

pub(super) fn consolidation_materialization_evidence(
	run_id: Uuid,
	fixture: &LiveConsolidationFixture,
	input_refs: &[ConsolidationInputRef],
	reviewed: &[ConsolidationProposalResponse],
) -> ConsolidationMaterializationEvidence {
	let review_actions = reviewed
		.iter()
		.flat_map(|proposal| proposal.review_events.iter().map(|event| event.action.clone()))
		.collect::<Vec<_>>();
	let final_review_states =
		reviewed.iter().map(|proposal| proposal.review_state.clone()).collect::<Vec<_>>();
	let unsupported_claim_flag_count = fixture
		.proposals
		.iter()
		.map(|proposal| {
			proposal.unsupported_claim_count.max(proposal.unsupported_claim_flags.len())
		})
		.sum();
	let review_event_count =
		reviewed.iter().map(|proposal| proposal.review_events.len()).sum::<usize>();

	ConsolidationMaterializationEvidence {
		run_id: Some(run_id),
		proposal_ids: reviewed.iter().map(|proposal| proposal.proposal_id).collect(),
		source_lineage_count: input_refs.len(),
		unsupported_claim_flag_count,
		review_event_count,
		review_actions,
		final_review_states,
	}
}

fn consolidation_input_refs(
	loaded: &LoadedJob,
	adapter_id: &str,
	evidence_ids: &[String],
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
) -> color_eyre::Result<Vec<ConsolidationInputRef>> {
	evidence_ids
		.iter()
		.map(|evidence_id| {
			let note_id = ingested
				.note_ids_by_evidence
				.get(evidence_id)
				.and_then(|ids| ids.first().copied())
				.ok_or_else(|| {
					eyre::eyre!(
						"No live note id mapped for consolidation evidence {} in {}.",
						evidence_id,
						loaded.job.job_id
					)
				})?;
			let text = corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| item.text.as_str())
				.unwrap_or(evidence_id.as_str());
			let content_hash = format!("blake3:{}", blake3::hash(text.as_bytes()).to_hex());

			Ok(ConsolidationInputRef {
				kind: ConsolidationSourceKind::Note,
				id: note_id,
				snapshot: ConsolidationSourceSnapshot {
					status: Some("active".to_string()),
					updated_at: Some(OffsetDateTime::now_utc()),
					content_hash: Some(content_hash),
					embedding_version: None,
					trace_version: None,
					source_ref: serde_json::json!({
						"schema": "real_world_live_adapter/v1",
						"adapter": adapter_id,
						"job_id": loaded.job.job_id,
						"evidence_id": evidence_id
					}),
					metadata: serde_json::json!({
						"evidence_id": evidence_id,
						"source": "memory_notes"
					}),
				},
			})
		})
		.collect()
}

fn push_unique_input_ref(values: &mut Vec<ConsolidationInputRef>, value: ConsolidationInputRef) {
	if !values.iter().any(|existing| existing.id == value.id) {
		values.push(value);
	}
}

fn consolidation_unsupported_claim_flags(
	loaded: &LoadedJob,
	adapter_id: &str,
	proposal: &LiveConsolidationProposal,
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
) -> color_eyre::Result<Vec<ConsolidationUnsupportedClaimFlag>> {
	proposal
		.unsupported_claim_flags
		.iter()
		.map(|flag| {
			let source = flag
				.source_ref
				.as_deref()
				.map(|source_ref| {
					consolidation_input_refs(
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

fn consolidation_diff(value: serde_json::Value) -> color_eyre::Result<ConsolidationProposalDiff> {
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

pub(super) fn consolidation_review_action(
	raw: &str,
) -> color_eyre::Result<ConsolidationReviewAction> {
	match raw {
		"apply" => Ok(ConsolidationReviewAction::Apply),
		"discard" => Ok(ConsolidationReviewAction::Discard),
		"defer" => Ok(ConsolidationReviewAction::Defer),
		"approve" => Ok(ConsolidationReviewAction::Approve),
		_ => Err(eyre::eyre!("Unknown consolidation review action {raw}.")),
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

pub(super) fn live_consolidation_response(
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> color_eyre::Result<serde_json::Value> {
	let proposals = fixture
		.proposals
		.iter()
		.zip(reviewed)
		.map(|(fixture_proposal, reviewed_proposal)| {
			serde_json::json!({
				"proposal_id": reviewed_proposal.proposal_id.to_string(),
				"proposal_kind": fixture_proposal.proposal_kind.clone(),
				"source_refs": fixture_proposal.source_refs.clone(),
				"expected_source_refs": if fixture_proposal.expected_source_refs.is_empty() {
					fixture_proposal.source_refs.clone()
				} else {
					fixture_proposal.expected_source_refs.clone()
				},
				"usefulness_score": fixture_proposal.usefulness_score,
				"min_usefulness_score": fixture_proposal.min_usefulness_score,
				"expected_review_action": fixture_proposal.expected_review_action.clone(),
				"actual_review_action": fixture_proposal.actual_review_action.clone(),
				"source_mutations": fixture_proposal.source_mutations.clone(),
				"unsupported_claim_count": fixture_proposal
					.unsupported_claim_count
					.max(fixture_proposal.unsupported_claim_flags.len()),
				"unsupported_claim_flags": fixture_proposal.unsupported_claim_flags.clone(),
				"diff": fixture_proposal.diff.clone(),
				"live_review_state": reviewed_proposal.review_state.clone(),
				"live_review_event_count": reviewed_proposal.review_events.len()
			})
		})
		.collect::<Vec<_>>();

	Ok(serde_json::json!({ "proposals": proposals, "executable_gaps": [] }))
}

pub(super) fn live_note_ids(ingested: &IngestedCorpus) -> Vec<Uuid> {
	let mut note_ids = Vec::new();

	for ids in ingested.note_ids_by_evidence.values() {
		for note_id in ids {
			if !note_ids.iter().any(|existing| existing == note_id) {
				note_ids.push(*note_id);
			}
		}
	}

	note_ids
}
