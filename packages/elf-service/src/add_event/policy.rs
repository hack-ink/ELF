use super::types::{AddEventResult, NoteProcessingData};
use crate::{NoteOp, UpdateDecision};
use elf_config::Config;
use elf_domain::memory_policy::{self, MemoryPolicyDecision};

const IGNORE_DUPLICATE: &str = "IGNORE_DUPLICATE";
const IGNORE_POLICY_THRESHOLD: &str = "IGNORE_POLICY_THRESHOLD";

pub(super) fn resolve_policy_for_update(
	cfg: &Config,
	note_data: &NoteProcessingData,
	base_decision: MemoryPolicyDecision,
) -> (MemoryPolicyDecision, Option<String>, Option<f32>, Option<f32>) {
	if matches!(base_decision, MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update) {
		let policy_eval = memory_policy::evaluate_memory_policy(
			cfg,
			note_data.note_type.as_str(),
			note_data.scope.as_str(),
			note_data.confidence as f64,
			note_data.importance as f64,
			base_decision,
		);
		let decision_policy_rule = policy_eval
			.matched_rule
			.and_then(|rule| policy_rule_id(rule.note_type.as_deref(), rule.scope.as_deref()));
		let min_confidence = policy_eval.matched_rule.and_then(|rule| rule.min_confidence);
		let min_importance = policy_eval.matched_rule.and_then(|rule| rule.min_importance);

		(policy_eval.decision, decision_policy_rule, min_confidence, min_importance)
	} else {
		(MemoryPolicyDecision::Ignore, None, None, None)
	}
}

pub(super) fn ignore_reason_code_for_policy(
	base_decision: MemoryPolicyDecision,
	policy_decision: MemoryPolicyDecision,
	matched_duplicate: bool,
) -> Option<&'static str> {
	if !matches!(policy_decision, MemoryPolicyDecision::Ignore) {
		return None;
	}

	match base_decision {
		MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update =>
			Some(IGNORE_POLICY_THRESHOLD),
		MemoryPolicyDecision::Ignore if matched_duplicate => Some(IGNORE_DUPLICATE),
		_ => None,
	}
}

pub(super) fn build_result_from_decision(
	decision: &UpdateDecision,
	policy_decision: MemoryPolicyDecision,
	reason: Option<String>,
	structured_present: bool,
) -> AddEventResult {
	match decision {
		UpdateDecision::Add { note_id, .. } => AddEventResult {
			note_id: Some(*note_id),
			op: NoteOp::Add,
			policy_decision,
			reason_code: None,
			reason,
			field_path: None,
			write_policy_audits: None,
		},
		UpdateDecision::Update { note_id, .. } => AddEventResult {
			note_id: Some(*note_id),
			op: NoteOp::Update,
			policy_decision,
			reason_code: None,
			reason,
			field_path: None,
			write_policy_audits: None,
		},
		UpdateDecision::None { note_id, .. } => AddEventResult {
			note_id: Some(*note_id),
			op: if structured_present { NoteOp::Update } else { NoteOp::None },
			policy_decision,
			reason_code: None,
			reason,
			field_path: None,
			write_policy_audits: None,
		},
	}
}

pub(super) fn apply_policy_ignore_adjustments(
	result: &mut AddEventResult,
	decision: &UpdateDecision,
	policy_decision: MemoryPolicyDecision,
	ignore_reason_code: Option<&str>,
) {
	if !matches!(policy_decision, MemoryPolicyDecision::Ignore) {
		return;
	}

	if let UpdateDecision::Add { .. } = decision {
		result.note_id = None;
	}

	result.op = NoteOp::None;
	result.reason_code = ignore_reason_code.map(str::to_string);
}

pub(super) fn base_decision_for_update(
	decision: &UpdateDecision,
	structured_present: bool,
	graph_present: bool,
) -> MemoryPolicyDecision {
	match decision {
		UpdateDecision::Update { .. } => MemoryPolicyDecision::Update,
		UpdateDecision::Add { .. } => MemoryPolicyDecision::Remember,
		UpdateDecision::None { .. } =>
			if structured_present || graph_present {
				MemoryPolicyDecision::Update
			} else {
				MemoryPolicyDecision::Ignore
			},
	}
}

fn policy_rule_id(note_type: Option<&str>, scope: Option<&str>) -> Option<String> {
	match (note_type, scope) {
		(Some(note_type), Some(scope)) => Some(format!("note_type={note_type},scope={scope}")),
		(Some(note_type), None) => Some(format!("note_type={note_type}")),
		(None, Some(scope)) => Some(format!("scope={scope}")),
		(None, None) => None,
	}
}
