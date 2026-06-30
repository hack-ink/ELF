use crate::evidence_selection::{self, LiveMemoryEvolution, LoadedJob};

pub(super) fn temporal_reconciliation_relevant_ids(
	loaded: &LoadedJob,
	evolution: &LiveMemoryEvolution,
) -> Vec<String> {
	let mut ids = Vec::new();

	for evidence in &loaded.job.required_evidence {
		evidence_selection::push_unique(&mut ids, evidence.evidence_id.clone());
	}
	for evidence_id in &evolution.current_evidence_ids {
		evidence_selection::push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.historical_evidence_ids {
		evidence_selection::push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.tombstone_evidence_ids {
		evidence_selection::push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.invalidation_evidence_ids {
		evidence_selection::push_unique(&mut ids, evidence_id.clone());
	}
	for conflict in &evolution.conflicts {
		evidence_selection::push_unique(&mut ids, conflict.current_evidence_id.clone());
		evidence_selection::push_unique(&mut ids, conflict.historical_evidence_id.clone());

		if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
			evidence_selection::push_unique(&mut ids, evidence_id.clone());
		}
	}

	if let Some(rationale) = &evolution.update_rationale
		&& rationale.available
	{
		for evidence_id in &rationale.evidence_ids {
			evidence_selection::push_unique(&mut ids, evidence_id.clone());
		}
	}

	ids
}
