use crate::{
	Value,
	evidence_selection::{
		self, BTreeSet, IngestedCorpus, LiveMemoryEvolution, LoadedJob,
		TemporalReconciliationMaterializationEvidence, common,
	},
};

pub(super) fn temporal_reconciliation_evidence(
	evolution: &LiveMemoryEvolution,
	relevant_ids: &[String],
	retrieved_evidence_ids: &[String],
	selected_ids: &[String],
	ingested: &IngestedCorpus,
	loaded: &LoadedJob,
) -> TemporalReconciliationMaterializationEvidence {
	let selected = selected_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let retrieved = retrieved_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let mut evidence = TemporalReconciliationMaterializationEvidence {
		current_winner_evidence_ids: selected_subset(&evolution.current_evidence_ids, &selected),
		historical_loser_evidence_ids: selected_subset(
			&evolution.historical_evidence_ids,
			&selected,
		),
		supersession_rationale_evidence_ids: evolution
			.update_rationale
			.as_ref()
			.filter(|rationale| rationale.available)
			.map_or_else(Vec::new, |rationale| selected_subset(&rationale.evidence_ids, &selected)),
		tombstone_evidence_ids: selected_subset(&evolution.tombstone_evidence_ids, &selected),
		invalidation_evidence_ids: selected_subset(&evolution.invalidation_evidence_ids, &selected),
		conflict_candidate_evidence_ids: conflict_candidate_ids(evolution, &selected),
		retrieved_evidence_ids: retrieved_evidence_ids.to_vec(),
		selected_evidence_ids: selected_ids.to_vec(),
		absent_evidence_ids: relevant_ids
			.iter()
			.filter(|id| !ingested.note_ids_by_evidence.contains_key(*id))
			.cloned()
			.collect(),
		retrieved_but_dropped_evidence_ids: relevant_ids
			.iter()
			.filter(|id| retrieved.contains(id.as_str()) && !selected.contains(id.as_str()))
			.cloned()
			.collect(),
		selected_but_not_narrated_evidence_ids: selected_but_not_narrated_ids(loaded, selected_ids),
		contradicted_by_lifecycle_evidence_ids: Vec::new(),
	};

	for evidence_id in evidence
		.historical_loser_evidence_ids
		.iter()
		.chain(evidence.tombstone_evidence_ids.iter())
		.chain(evidence.invalidation_evidence_ids.iter())
	{
		evidence_selection::push_unique(
			&mut evidence.contradicted_by_lifecycle_evidence_ids,
			evidence_id.clone(),
		);
	}

	evidence
}

pub(super) fn selected_subset(ids: &[String], selected: &BTreeSet<&str>) -> Vec<String> {
	ids.iter().filter(|id| selected.contains(id.as_str())).cloned().collect()
}

fn conflict_candidate_ids(
	evolution: &LiveMemoryEvolution,
	selected: &BTreeSet<&str>,
) -> Vec<String> {
	let mut ids = Vec::new();

	for conflict in &evolution.conflicts {
		common::push_if_selected(&mut ids, conflict.current_evidence_id.as_str(), selected);
		common::push_if_selected(&mut ids, conflict.historical_evidence_id.as_str(), selected);

		if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
			common::push_if_selected(&mut ids, evidence_id.as_str(), selected);
		}
	}

	ids
}

fn selected_but_not_narrated_ids(loaded: &LoadedJob, selected_ids: &[String]) -> Vec<String> {
	let claims = evidence_selection::temporal_reconciliation_claims(loaded, selected_ids);
	let narrated = claims
		.iter()
		.flat_map(|claim| {
			claim
				.get("evidence_ids")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(Value::as_str)
		})
		.collect::<BTreeSet<_>>();

	selected_ids.iter().filter(|id| !narrated.contains(id.as_str())).cloned().collect()
}
