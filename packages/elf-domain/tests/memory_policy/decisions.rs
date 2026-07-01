use crate::support;
use elf_config::{MemoryPolicy, MemoryPolicyRule};
use elf_domain::memory_policy::{self, MemoryPolicyDecision};

#[test]
fn downgrades_only_remember_or_update() {
	let cfg = support::memory_policy_config(MemoryPolicy {
		rules: vec![MemoryPolicyRule {
			note_type: Some("fact".to_string()),
			scope: Some("agent_private".to_string()),
			min_confidence: Some(0.9),
			min_importance: None,
		}],
	});
	let remember = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Remember,
	);

	assert_eq!(remember.decision, MemoryPolicyDecision::Ignore);

	let update = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Update,
	);

	assert_eq!(update.decision, MemoryPolicyDecision::Ignore);

	let ignored = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Ignore,
	);

	assert_eq!(ignored.decision, MemoryPolicyDecision::Ignore);

	let rejected = memory_policy::evaluate_memory_policy(
		&cfg,
		"fact",
		"agent_private",
		0.5,
		0.5,
		MemoryPolicyDecision::Reject,
	);

	assert_eq!(rejected.decision, MemoryPolicyDecision::Reject);
}
