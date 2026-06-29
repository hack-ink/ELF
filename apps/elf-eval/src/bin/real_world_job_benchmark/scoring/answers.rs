use crate::scoring::{BTreeSet, ExpectedClaim, ProducedAnswer, RealWorldJob, RequiredEvidence};

pub(super) fn produced_answer(job: &RealWorldJob) -> &ProducedAnswer {
	job.corpus
		.adapter_response
		.as_ref()
		.map(|response| &response.answer)
		.unwrap_or_else(|| synthetic_answer(job))
}

pub(super) fn produced_evidence_ids(answer: &ProducedAnswer) -> BTreeSet<String> {
	ordered_produced_evidence_ids(answer).into_iter().collect()
}

pub(super) fn missing_required_claims(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<String> {
	job.expected_answer
		.must_include
		.iter()
		.filter(|claim| !claim_is_present(claim, answer))
		.map(|claim| claim.text().to_string())
		.collect()
}

pub(super) fn forbidden_claim_hits(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<String> {
	job.expected_answer
		.must_not_include
		.iter()
		.filter(|claim| answer.content.contains(claim.as_str()))
		.cloned()
		.collect()
}

pub(super) fn missing_required_evidence(
	job: &RealWorldJob,
	produced_evidence: &BTreeSet<String>,
) -> Vec<String> {
	job.required_evidence
		.iter()
		.filter(|evidence| {
			is_required_use(evidence) && !produced_evidence.contains(&evidence.evidence_id)
		})
		.map(|evidence| evidence.evidence_id.clone())
		.collect()
}

pub(super) fn is_required_use(evidence: &RequiredEvidence) -> bool {
	matches!(evidence.requirement.as_str(), "cite" | "use" | "explain")
}

pub(super) fn trap_ids_used(
	job: &RealWorldJob,
	produced_evidence: &BTreeSet<String>,
) -> Vec<String> {
	job.negative_traps
		.iter()
		.filter(|trap| trap.failure_if_used)
		.filter(|trap| {
			trap.evidence_ids.iter().any(|evidence_id| produced_evidence.contains(evidence_id))
		})
		.map(|trap| trap.trap_id.clone())
		.collect()
}

fn synthetic_answer(job: &RealWorldJob) -> &ProducedAnswer {
	let _ = job;

	static EMPTY_ANSWER: std::sync::OnceLock<ProducedAnswer> = std::sync::OnceLock::new();

	EMPTY_ANSWER.get_or_init(|| ProducedAnswer {
		content: String::new(),
		claims: Vec::new(),
		evidence_ids: Vec::new(),
		pages: Vec::new(),
		memory_summaries: Vec::new(),
		proactive_briefs: Vec::new(),
		scheduled_tasks: Vec::new(),
		work_journal_readbacks: Vec::new(),
		recovery_drills: Vec::new(),
		latency_ms: None,
		cost: None,
		trace_explainability: None,
	})
}

fn ordered_produced_evidence_ids(answer: &ProducedAnswer) -> Vec<String> {
	let mut seen = BTreeSet::new();
	let mut evidence = Vec::new();

	for evidence_id in &answer.evidence_ids {
		push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
	}
	for claim in &answer.claims {
		for evidence_id in &claim.evidence_ids {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
	}
	for brief in &answer.proactive_briefs {
		for suggestion in &brief.suggestions {
			for evidence_id in &suggestion.evidence_refs {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
		}
	}
	for task in &answer.scheduled_tasks {
		for output in &task.outputs {
			for evidence_id in &output.evidence_refs {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
		}
	}
	for readback in &answer.work_journal_readbacks {
		for entry in &readback.items {
			for evidence_id in &entry.source_refs {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
			for step in entry.explicit_next_steps.iter().chain(entry.inferred_next_steps.iter()) {
				for evidence_id in &step.evidence_refs {
					push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
				}
			}
			for option in &entry.rejected_options {
				for evidence_id in &option.evidence_refs {
					push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
				}
			}
		}

		if let Some(where_stopped) = &readback.where_stopped {
			for evidence_id in &where_stopped.decision_rationale_evidence_ids {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
			for evidence_id in &where_stopped.handoff_source_refs {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
		}

		for candidate in &readback.janitor_candidates {
			for evidence_id in &candidate.evidence_refs {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
		}
	}
	for drill in &answer.recovery_drills {
		for evidence_id in &drill.backup_pitr.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for evidence_id in &drill.degraded_read.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for evidence_id in &drill.rpo.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for evidence_id in &drill.rto.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for evidence_id in &drill.outbox_replay.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for evidence_id in &drill.qdrant_rebuild.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for evidence_id in &drill.migration_repair.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for evidence_id in &drill.dead_letter.evidence_refs {
			push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
		}
		for injection in &drill.failure_injections {
			for evidence_id in &injection.evidence_refs {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
		}
		for count in &drill.authority_record_counts {
			for evidence_id in &count.evidence_refs {
				push_ordered_evidence(&mut evidence, &mut seen, evidence_id);
			}
		}
	}

	evidence
}

fn push_ordered_evidence(
	evidence: &mut Vec<String>,
	seen: &mut BTreeSet<String>,
	evidence_id: &str,
) {
	if seen.insert(evidence_id.to_string()) {
		evidence.push(evidence_id.to_string());
	}
}

fn claim_is_present(claim: &ExpectedClaim, answer: &ProducedAnswer) -> bool {
	if let Some(claim_id) = claim.claim_id()
		&& answer.claims.iter().any(|produced| produced.claim_id.as_deref() == Some(claim_id))
	{
		return true;
	}

	answer.content.contains(claim.text())
}
