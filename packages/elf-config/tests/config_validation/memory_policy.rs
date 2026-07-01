use crate::helpers;
use elf_config::MemoryPolicyRule;

#[test]
fn memory_policy_min_confidence_must_be_finite() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { min_confidence: Some(f32::NAN), ..Default::default() });

	let err = elf_config::validate(&cfg).expect_err("Expected min_confidence validation error.");

	assert!(
		err.to_string().contains("memory.policy.rules[1].min_confidence must be a finite number."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_min_confidence_must_be_in_range() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { min_confidence: Some(1.01), ..Default::default() });

	let err =
		elf_config::validate(&cfg).expect_err("Expected min_confidence range validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].min_confidence must be between 0.0 and 1.0."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_min_importance_must_be_finite() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { min_importance: Some(f32::INFINITY), ..Default::default() });

	let err = elf_config::validate(&cfg).expect_err("Expected min_importance validation error.");

	assert!(
		err.to_string().contains("memory.policy.rules[1].min_importance must be a finite number."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_min_importance_must_be_in_range() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { min_importance: Some(-0.01), ..Default::default() });

	let err =
		elf_config::validate(&cfg).expect_err("Expected min_importance range validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].min_importance must be between 0.0 and 1.0."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_note_type_must_be_known_value() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { note_type: Some("unknown".to_string()), ..Default::default() });

	let err = elf_config::validate(&cfg).expect_err("Expected note_type validation error.");

	assert!(
		err.to_string().contains(
			"memory.policy.rules[1].note_type must be one of preference, constraint, decision, profile, fact, or plan."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_scope_must_be_allowed() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { scope: Some("invalid_scope".to_string()), ..Default::default() });

	let err = elf_config::validate(&cfg).expect_err("Expected scope validation error.");

	assert!(
		err.to_string().contains("memory.policy.rules[1].scope must be one of allowed scopes."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_rule_pairs_must_be_unique() {
	let mut cfg = helpers::base_config();

	cfg.memory.policy.rules.push(Default::default());
	cfg.memory.policy.rules.push(Default::default());

	let err = elf_config::validate(&cfg).expect_err("Expected duplicate rule validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[2] has a duplicate note_type and scope pair."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_note_type_must_not_be_whitespace_only() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { note_type: Some("   ".to_string()), ..Default::default() });

	let err =
		elf_config::validate(&cfg).expect_err("Expected whitespace note_type validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].note_type cannot be blank or whitespace-only."),
		"Unexpected error: {err}"
	);
}

#[test]
fn memory_policy_scope_must_not_be_whitespace_only() {
	let mut cfg = helpers::base_config();

	cfg.memory
		.policy
		.rules
		.push(MemoryPolicyRule { scope: Some("   ".to_string()), ..Default::default() });

	let err = elf_config::validate(&cfg).expect_err("Expected whitespace scope validation error.");

	assert!(
		err.to_string()
			.contains("memory.policy.rules[1].scope cannot be blank or whitespace-only."),
		"Unexpected error: {err}"
	);
}
