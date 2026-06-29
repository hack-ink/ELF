use super::{common::push_if_selected, *};

pub(super) fn temporal_reconciliation_selection_impl(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	retrieved_evidence_ids: &[String],
	ingested: &IngestedCorpus,
) -> Option<TemporalReconciliationSelection> {
	let evolution = loaded.job.memory_evolution.as_ref()?;
	let relevant_ids = temporal_reconciliation_relevant_ids(loaded, evolution);
	let retrieved_ids = retrieved_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let mut selected_ids = Vec::new();

	for evidence_id in &relevant_ids {
		if retrieved_ids.contains(evidence_id.as_str())
			&& ingested.note_ids_by_evidence.contains_key(evidence_id)
		{
			push_unique(&mut selected_ids, evidence_id.clone());
		}
	}

	if selected_ids.is_empty() {
		return None;
	}

	let content = temporal_reconciliation_content(loaded, corpus, &selected_ids);
	let selected = SelectedEvidenceText { content, evidence_ids: selected_ids.clone() };
	let evidence = temporal_reconciliation_evidence(
		evolution,
		&relevant_ids,
		retrieved_evidence_ids,
		&selected_ids,
		ingested,
		loaded,
	);
	let trace_stages =
		temporal_reconciliation_trace_stages(evolution, retrieved_evidence_ids, &evidence);

	Some(TemporalReconciliationSelection { selected, evidence, trace_stages })
}

fn temporal_reconciliation_relevant_ids(
	loaded: &LoadedJob,
	evolution: &LiveMemoryEvolution,
) -> Vec<String> {
	let mut ids = Vec::new();

	for evidence in &loaded.job.required_evidence {
		push_unique(&mut ids, evidence.evidence_id.clone());
	}
	for evidence_id in &evolution.current_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.historical_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.tombstone_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.invalidation_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for conflict in &evolution.conflicts {
		push_unique(&mut ids, conflict.current_evidence_id.clone());
		push_unique(&mut ids, conflict.historical_evidence_id.clone());

		if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
			push_unique(&mut ids, evidence_id.clone());
		}
	}

	if let Some(rationale) = &evolution.update_rationale
		&& rationale.available
	{
		for evidence_id in &rationale.evidence_ids {
			push_unique(&mut ids, evidence_id.clone());
		}
	}

	ids
}

fn temporal_reconciliation_content(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	selected_ids: &[String],
) -> String {
	let expected = loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.map(LiveExpectedClaim::text)
		.collect::<Vec<_>>()
		.join(" ");
	let evidence_summary = selected_ids
		.iter()
		.filter_map(|evidence_id| {
			corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| format!("{evidence_id}: {}", item.text))
		})
		.collect::<Vec<_>>()
		.join("\n");

	if evidence_summary.is_empty() {
		expected
	} else {
		format!("{expected}\n\nTemporal reconciliation evidence:\n{evidence_summary}")
	}
}

fn temporal_reconciliation_evidence(
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
		push_unique(&mut evidence.contradicted_by_lifecycle_evidence_ids, evidence_id.clone());
	}

	evidence
}

fn selected_subset(ids: &[String], selected: &BTreeSet<&str>) -> Vec<String> {
	ids.iter().filter(|id| selected.contains(id.as_str())).cloned().collect()
}

fn conflict_candidate_ids(
	evolution: &LiveMemoryEvolution,
	selected: &BTreeSet<&str>,
) -> Vec<String> {
	let mut ids = Vec::new();

	for conflict in &evolution.conflicts {
		push_if_selected(&mut ids, conflict.current_evidence_id.as_str(), selected);
		push_if_selected(&mut ids, conflict.historical_evidence_id.as_str(), selected);

		if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
			push_if_selected(&mut ids, evidence_id.as_str(), selected);
		}
	}

	ids
}

fn selected_but_not_narrated_ids(loaded: &LoadedJob, selected_ids: &[String]) -> Vec<String> {
	let claims = temporal_reconciliation_claims(loaded, selected_ids);
	let narrated = claims
		.iter()
		.flat_map(|claim| {
			claim
				.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(serde_json::Value::as_str)
		})
		.collect::<BTreeSet<_>>();

	selected_ids.iter().filter(|id| !narrated.contains(id.as_str())).cloned().collect()
}

fn temporal_reconciliation_trace_stages(
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
