//! Memory-policy evaluation helpers.

use serde::{Deserialize, Serialize};

use elf_config::{Config, MemoryPolicyRule};

/// Base memory decision after policy evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPolicyDecision {
	/// Persist the note as a new memory item.
	Remember,
	/// Update an existing memory item.
	Update,
	/// Ignore the note without persisting it.
	Ignore,
	/// Reject the note entirely.
	Reject,
}

/// Result of evaluating memory-policy rules for one note candidate.
#[derive(Debug)]
pub struct MemoryPolicyEvaluation<'a> {
	/// Final decision after any downgrade rules are applied.
	pub decision: MemoryPolicyDecision,
	/// Rule that matched the note, if any.
	pub matched_rule: Option<&'a MemoryPolicyRule>,
}

/// Evaluates memory-policy downgrade rules for a note candidate.
pub fn evaluate_memory_policy<'a>(
	cfg: &'a Config,
	note_type: &str,
	scope: &str,
	confidence: f64,
	importance: f64,
	base_decision: MemoryPolicyDecision,
) -> MemoryPolicyEvaluation<'a> {
	let matched_rule = select_memory_policy_rule(cfg, note_type, scope);
	let decision =
		if matches!(base_decision, MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update)
			&& should_downgrade(matched_rule, confidence, importance)
		{
			MemoryPolicyDecision::Ignore
		} else {
			base_decision
		};

	MemoryPolicyEvaluation { decision, matched_rule }
}

fn select_memory_policy_rule<'a>(
	cfg: &'a Config,
	note_type: &str,
	scope: &str,
) -> Option<&'a MemoryPolicyRule> {
	let exact_match =
		cfg.memory.policy.rules.iter().find(|rule| matches_exact(note_type, scope, rule));

	if exact_match.is_some() {
		return exact_match;
	}

	let note_type_match =
		cfg.memory.policy.rules.iter().find(|rule| matches_note_type(note_type, rule));

	if note_type_match.is_some() {
		return note_type_match;
	}

	let scope_match = cfg.memory.policy.rules.iter().find(|rule| matches_scope(scope, rule));

	if scope_match.is_some() {
		return scope_match;
	}

	cfg.memory.policy.rules.iter().find(|rule| rule.note_type.is_none() && rule.scope.is_none())
}

fn matches_exact(note_type: &str, scope: &str, rule: &MemoryPolicyRule) -> bool {
	match (rule.note_type.as_deref(), rule.scope.as_deref()) {
		(Some(rule_type), Some(rule_scope)) => rule_type == note_type && rule_scope == scope,
		_ => false,
	}
}

fn matches_note_type(note_type: &str, rule: &MemoryPolicyRule) -> bool {
	match (rule.note_type.as_deref(), rule.scope.as_deref()) {
		(Some(rule_type), None) => rule_type == note_type,
		_ => false,
	}
}

fn matches_scope(scope: &str, rule: &MemoryPolicyRule) -> bool {
	match (rule.note_type.as_deref(), rule.scope.as_deref()) {
		(None, Some(rule_scope)) => rule_scope == scope,
		_ => false,
	}
}

fn should_downgrade(
	matched_rule: Option<&MemoryPolicyRule>,
	confidence: f64,
	importance: f64,
) -> bool {
	let Some(rule) = matched_rule else {
		return false;
	};

	if let Some(min_confidence) = rule.min_confidence
		&& (!confidence.is_finite() || confidence < f64::from(min_confidence))
	{
		return true;
	}
	if let Some(min_importance) = rule.min_importance
		&& (!importance.is_finite() || importance < f64::from(min_importance))
	{
		return true;
	}

	false
}

#[cfg(test)]
#[path = "memory_policy/tests.rs"]
mod tests;
