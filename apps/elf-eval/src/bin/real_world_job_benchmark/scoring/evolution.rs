use super::*;

pub(super) fn evolution_job_report(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
	trap_ids_used: &[String],
	forbidden_claim_count: usize,
) -> Option<EvolutionJobReport> {
	let evolution = job.memory_evolution.as_ref()?;
	let produced = produced_evidence_ids(answer);
	let stale_trap_ids_used = stale_trap_ids_used(job, evolution, trap_ids_used);
	let stale_answer_count =
		stale_answer_count(job, evolution, &stale_trap_ids_used, forbidden_claim_count);
	let conflict_detection_count = evolution
		.conflicts
		.iter()
		.filter(|conflict| conflict_is_detected(conflict, answer))
		.count();
	let update_rationale_available = evolution
		.update_rationale
		.as_ref()
		.is_some_and(|rationale| update_rationale_is_available(rationale, answer));
	let temporal_validity_required =
		evolution.temporal_validity.as_ref().is_some_and(|temporal| temporal.required);
	let temporal_validity_encoded =
		evolution.temporal_validity.as_ref().is_some_and(|temporal| temporal.encoded);
	let temporal_validity_not_encoded = temporal_validity_required && !temporal_validity_encoded;
	let history_readback_encoded =
		evolution.history_readback.as_ref().is_some_and(|history| history.encoded);
	let history_event_types = evolution
		.history_readback
		.as_ref()
		.map_or_else(Vec::new, |history| history.required_event_types.clone());
	let history_requires_note_version_links = evolution
		.history_readback
		.as_ref()
		.is_some_and(|history| history.requires_note_version_links);
	let follow_up = evolution
		.temporal_validity
		.as_ref()
		.and_then(|temporal| temporal.follow_up.clone())
		.or_else(|| job.encoding.follow_up.as_ref().map(|follow_up| follow_up.title.clone()));

	Some(EvolutionJobReport {
		current_evidence: evolution.current_evidence_ids.clone(),
		historical_evidence: evolution.historical_evidence_ids.clone(),
		tombstone_evidence: evolution.tombstone_evidence_ids.clone(),
		invalidation_evidence: evolution.invalidation_evidence_ids.clone(),
		selected_current_evidence: selected_evolution_evidence(
			&evolution.current_evidence_ids,
			&produced,
		),
		selected_historical_evidence: selected_evolution_evidence(
			&evolution.historical_evidence_ids,
			&produced,
		),
		selected_rationale_evidence: selected_rationale_evidence(evolution, &produced),
		selected_tombstone_evidence: selected_evolution_evidence(
			&evolution.tombstone_evidence_ids,
			&produced,
		),
		selected_invalidation_evidence: selected_evolution_evidence(
			&evolution.invalidation_evidence_ids,
			&produced,
		),
		conflict_candidate_evidence: selected_conflict_candidate_evidence(evolution, &produced),
		retrieved_but_dropped_evidence: trace_dropped_evidence(answer),
		selected_but_not_narrated_evidence: selected_but_not_narrated_evidence(answer),
		stale_answer_count,
		stale_trap_ids_used,
		conflict_count: evolution.conflicts.len(),
		conflict_detection_count,
		update_rationale_available,
		temporal_validity_required,
		temporal_validity_encoded,
		temporal_validity_not_encoded,
		history_readback_encoded,
		history_event_types,
		history_requires_note_version_links,
		follow_up,
	})
}

fn stale_answer_count(
	job: &RealWorldJob,
	evolution: &MemoryEvolution,
	stale_trap_ids_used: &[String],
	forbidden_claim_count: usize,
) -> usize {
	let stale_trap_count = if evolution.stale_trap_ids.is_empty() {
		job.negative_traps.iter().filter(|trap| trap.trap_type == "stale_fact").count()
	} else {
		evolution.stale_trap_ids.len()
	};
	let stale_forbidden_claims = if stale_trap_count > 0 { forbidden_claim_count } else { 0 };

	stale_trap_ids_used.len().max(stale_forbidden_claims)
}

fn selected_evolution_evidence(
	evidence_ids: &[String],
	produced: &BTreeSet<String>,
) -> Vec<String> {
	evidence_ids.iter().filter(|evidence_id| produced.contains(*evidence_id)).cloned().collect()
}

fn selected_rationale_evidence(
	evolution: &MemoryEvolution,
	produced: &BTreeSet<String>,
) -> Vec<String> {
	evolution.update_rationale.as_ref().map_or_else(Vec::new, |rationale| {
		selected_evolution_evidence(&rationale.evidence_ids, produced)
	})
}

fn selected_conflict_candidate_evidence(
	evolution: &MemoryEvolution,
	produced: &BTreeSet<String>,
) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	for conflict in &evolution.conflicts {
		push_if_produced(&mut evidence_ids, conflict.current_evidence_id.as_str(), produced);
		push_if_produced(&mut evidence_ids, conflict.historical_evidence_id.as_str(), produced);

		if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
			push_if_produced(&mut evidence_ids, evidence_id.as_str(), produced);
		}
	}

	evidence_ids
}

fn push_if_produced(out: &mut Vec<String>, evidence_id: &str, produced: &BTreeSet<String>) {
	if produced.contains(evidence_id) && !out.iter().any(|id| id == evidence_id) {
		out.push(evidence_id.to_string());
	}
}

fn trace_dropped_evidence(answer: &ProducedAnswer) -> Vec<String> {
	let mut evidence = Vec::new();

	if let Some(trace) = &answer.trace_explainability {
		for stage in &trace.stages {
			for evidence_id in &stage.dropped_evidence {
				if !evidence.iter().any(|id| id == evidence_id) {
					evidence.push(evidence_id.clone());
				}
			}
		}
	}

	evidence
}

fn selected_but_not_narrated_evidence(answer: &ProducedAnswer) -> Vec<String> {
	let narrated = answer
		.claims
		.iter()
		.flat_map(|claim| claim.evidence_ids.iter().map(String::as_str))
		.collect::<BTreeSet<_>>();

	answer
		.evidence_ids
		.iter()
		.filter(|evidence_id| !narrated.contains(evidence_id.as_str()))
		.cloned()
		.collect()
}

fn stale_trap_ids_used(
	job: &RealWorldJob,
	evolution: &MemoryEvolution,
	trap_ids_used: &[String],
) -> Vec<String> {
	let declared_stale_traps = if evolution.stale_trap_ids.is_empty() {
		job.negative_traps
			.iter()
			.filter(|trap| trap.trap_type == "stale_fact")
			.map(|trap| trap.trap_id.as_str())
			.collect::<BTreeSet<_>>()
	} else {
		evolution.stale_trap_ids.iter().map(String::as_str).collect::<BTreeSet<_>>()
	};

	trap_ids_used
		.iter()
		.filter(|trap_id| declared_stale_traps.contains(trap_id.as_str()))
		.cloned()
		.collect()
}

fn conflict_is_detected(conflict: &EvolutionConflict, answer: &ProducedAnswer) -> bool {
	let mut required_evidence =
		vec![conflict.current_evidence_id.as_str(), conflict.historical_evidence_id.as_str()];

	if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
		required_evidence.push(evidence_id.as_str());
	}

	answer.claims.iter().any(|claim| {
		claim.claim_id.as_deref() == Some(conflict.claim_id.as_str())
			&& required_evidence
				.iter()
				.all(|evidence_id| claim.evidence_ids.iter().any(|id| id == evidence_id))
	})
}

fn update_rationale_is_available(rationale: &UpdateRationale, answer: &ProducedAnswer) -> bool {
	if !rationale.available {
		return false;
	}

	answer.claims.iter().any(|claim| {
		claim.claim_id.as_deref() == Some(rationale.claim_id.as_str())
			&& !claim.evidence_ids.is_empty()
			&& rationale.evidence_ids.iter().any(|evidence_id| {
				claim.evidence_ids.iter().any(|produced| produced == evidence_id)
			})
	})
}

pub(super) fn update_rationale_missing_count(report: &EvolutionJobReport) -> usize {
	if report.update_rationale_available || report.temporal_validity_not_encoded {
		0
	} else if report.conflict_count > 0 {
		1
	} else {
		0
	}
}
