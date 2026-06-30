use crate::evidence_selection::{
	BTreeSet, LiveMemoryEvolution, TemporalReconciliationMaterializationEvidence, TraceStageOutput,
};

pub(super) fn temporal_reconciliation_trace_stages(
	evolution: &LiveMemoryEvolution,
	retrieved_evidence_ids: &[String],
	evidence: &TemporalReconciliationMaterializationEvidence,
) -> Vec<TraceStageOutput> {
	let selected =
		evidence.selected_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let retrieved = retrieved_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let expected_not_retrieved = evidence
		.selected_evidence_ids
		.iter()
		.filter(|id| !retrieved.contains(id.as_str()))
		.cloned()
		.collect::<Vec<_>>();

	vec![
		TraceStageOutput {
			stage_name: "live_adapter.retrieve".to_string(),
			kept_evidence: retrieved_evidence_ids.to_vec(),
			dropped_evidence: expected_not_retrieved,
			demoted_evidence: Vec::new(),
			distractor_evidence: evidence.absent_evidence_ids.clone(),
			notes:
				"Search output is compared with the temporal reconciliation evidence contract."
					.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.current_winner".to_string(),
			kept_evidence: evidence.current_winner_evidence_ids.clone(),
			dropped_evidence: unselected_subset(&evolution.current_evidence_ids, &selected),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Current evidence selected as the answer winner.".to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.historical_loser".to_string(),
			kept_evidence: evidence.historical_loser_evidence_ids.clone(),
			dropped_evidence: unselected_subset(&evolution.historical_evidence_ids, &selected),
			demoted_evidence: evidence.historical_loser_evidence_ids.clone(),
			distractor_evidence: Vec::new(),
			notes: "Historical evidence preserved as history, not as the current answer."
				.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.supersession_rationale".to_string(),
			kept_evidence: evidence.supersession_rationale_evidence_ids.clone(),
			dropped_evidence: evolution
				.update_rationale
				.as_ref()
				.map_or_else(Vec::new, |rationale| {
					unselected_subset(&rationale.evidence_ids, &selected)
				}),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Rationale evidence selected to explain why the older fact was superseded."
				.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.tombstone_invalidation".to_string(),
			kept_evidence: evidence
				.tombstone_evidence_ids
				.iter()
				.chain(evidence.invalidation_evidence_ids.iter())
				.cloned()
				.collect(),
			dropped_evidence: evolution
				.tombstone_evidence_ids
				.iter()
				.chain(evolution.invalidation_evidence_ids.iter())
				.filter(|id| !selected.contains(id.as_str()))
				.cloned()
				.collect(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Tombstone or TTL invalidation evidence remains answerable when present."
				.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.conflict_candidates".to_string(),
			kept_evidence: evidence.conflict_candidate_evidence_ids.clone(),
			dropped_evidence: evidence.retrieved_but_dropped_evidence_ids.clone(),
			demoted_evidence: evidence.contradicted_by_lifecycle_evidence_ids.clone(),
			distractor_evidence: evidence.selected_but_not_narrated_evidence_ids.clone(),
			notes:
				"Conflict candidates record selected, dropped, non-narrated, and lifecycle-demoted evidence."
					.to_string(),
		},
	]
}

fn unselected_subset(ids: &[String], selected: &BTreeSet<&str>) -> Vec<String> {
	ids.iter().filter(|id| !selected.contains(id.as_str())).cloned().collect()
}
