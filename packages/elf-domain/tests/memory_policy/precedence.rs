use crate::support;
use elf_config::{MemoryPolicy, MemoryPolicyRule};
use elf_domain::memory_policy::{self, MemoryPolicyDecision, MemoryPolicyEvaluation};

#[test]
fn selects_note_type_and_scope_rule_before_note_type() {
	let cfg = support::memory_policy_config(MemoryPolicy {
		rules: vec![
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: None,
				min_confidence: Some(0.2),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.9),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: None,
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.0),
				min_importance: None,
			},
		],
	});
	let MemoryPolicyEvaluation { decision, matched_rule } = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(decision, MemoryPolicyDecision::Ignore);
	assert!(matched_rule.is_some());
	assert_eq!(matched_rule.unwrap().note_type.as_deref(), Some("fact"));
	assert_eq!(matched_rule.unwrap().scope.as_deref(), Some("agent_private"));
	assert_eq!(matched_rule.unwrap().min_confidence, Some(0.9));
}

#[test]
fn note_type_only_beats_scope_only() {
	let cfg = support::memory_policy_config(MemoryPolicy {
		rules: vec![
			MemoryPolicyRule {
				note_type: None,
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.1),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: None,
				min_confidence: Some(0.1),
				min_importance: None,
			},
		],
	});
	let output = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.2,
		0.0,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
	assert_eq!(output.matched_rule.and_then(|rule| rule.note_type.as_deref()), Some("fact"));
	assert_eq!(output.matched_rule.and_then(|rule| rule.scope.as_deref()), None);
}

#[test]
fn scope_only_beats_fallback_none() {
	let cfg = support::memory_policy_config(MemoryPolicy {
		rules: vec![
			MemoryPolicyRule {
				note_type: None,
				scope: None,
				min_confidence: Some(0.1),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: None,
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.1),
				min_importance: None,
			},
		],
	});
	let output = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.2,
		0.0,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
	assert_eq!(output.matched_rule.and_then(|rule| rule.note_type.as_deref()), None);
	assert_eq!(output.matched_rule.and_then(|rule| rule.scope.as_deref()), Some("agent_private"));
}
