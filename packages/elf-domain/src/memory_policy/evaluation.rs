use crate::memory_policy::{self, MemoryPolicyDecision, MemoryPolicyEvaluation, tests::support};
use elf_config::{MemoryPolicy, MemoryPolicyRule};

#[test]
fn policy_precedence_prefers_note_type_and_scope_over_note_type_only() {
	let cfg = support::test_config(MemoryPolicy {
		rules: vec![
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: None,
				min_confidence: Some(0.05),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: Some("fact".to_string()),
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.95),
				min_importance: None,
			},
			MemoryPolicyRule {
				note_type: None,
				scope: Some("agent_private".to_string()),
				min_confidence: Some(0.40),
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

	let rule = matched_rule.expect("expected policy match");

	assert_eq!(rule.note_type.as_deref(), Some("fact"));
	assert_eq!(rule.scope.as_deref(), Some("agent_private"));
	assert_eq!(rule.min_confidence, Some(0.95));
	assert_eq!(rule.min_importance, None);
}

#[test]
fn evaluate_downgrades_base_remember_update_only() {
	let cfg = support::test_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: Some(0.9),
			min_importance: Some(0.5),
		}],
	});
	let remember = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.95,
		0.4,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(remember.decision, MemoryPolicyDecision::Ignore);

	let update = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		f64::NAN,
		f64::NAN,
		MemoryPolicyDecision::Update,
	);

	assert_eq!(update.decision, MemoryPolicyDecision::Ignore);

	let ignore = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.1,
		0.1,
		MemoryPolicyDecision::Ignore,
	);

	assert_eq!(ignore.decision, MemoryPolicyDecision::Ignore);

	let reject = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.1,
		0.1,
		MemoryPolicyDecision::Reject,
	);

	assert_eq!(reject.decision, MemoryPolicyDecision::Reject);
}

#[test]
fn evaluate_without_matching_threshold_leaves_base_unchanged() {
	let cfg = support::test_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: None,
			min_importance: None,
		}],
	});
	let output = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.0,
		0.0,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
}
