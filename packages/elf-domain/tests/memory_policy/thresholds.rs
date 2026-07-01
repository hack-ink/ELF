use crate::support;
use elf_config::{MemoryPolicy, MemoryPolicyRule};
use elf_domain::memory_policy::{self, MemoryPolicyDecision};

#[test]
fn confidence_meets_minimum_is_not_a_downgrade() {
	let cfg = support::memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: Some(0.5),
			min_importance: None,
		}],
	});
	let output = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.0,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
}

#[test]
fn importance_meets_minimum_is_not_a_downgrade() {
	let cfg = support::memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: None,
			min_importance: Some(0.7),
		}],
	});
	let output = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.0,
		0.7,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Remember);
}

#[test]
fn missing_threshold_does_not_change_decision() {
	let cfg = support::memory_policy_config(MemoryPolicy {
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

#[test]
fn non_finite_metrics_fail_threshold() {
	let cfg = support::memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: Some(0.9),
			min_importance: None,
		}],
	});
	let output = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		f64::NAN,
		0.5,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(output.decision, MemoryPolicyDecision::Ignore);
}
