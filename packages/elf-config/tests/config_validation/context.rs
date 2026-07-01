use std::collections::HashMap;

use crate::helpers;
use elf_config::Context;

#[test]
fn context_scope_boost_weight_requires_scope_descriptions_when_enabled() {
	let mut cfg = helpers::base_config();

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: None,
		scope_boost_weight: Some(0.1),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains(
			"context.scope_descriptions must be non-empty when context.scope_boost_weight is greater than zero."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn context_scope_boost_weight_accepts_zero_without_descriptions() {
	let mut cfg = helpers::base_config();

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: None,
		scope_boost_weight: Some(0.0),
	});

	assert!(elf_config::validate(&cfg).is_ok());
}

#[test]
fn context_scope_boost_weight_must_be_finite() {
	let mut cfg = helpers::base_config();
	let mut scope_descriptions = HashMap::new();

	scope_descriptions.insert("project_shared".to_string(), "Project notes.".to_string());

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: Some(scope_descriptions),
		scope_boost_weight: Some(f32::NAN),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains("context.scope_boost_weight must be a finite number."),
		"Unexpected error: {err}"
	);
}

#[test]
fn context_scope_boost_weight_must_be_in_range() {
	let mut cfg = helpers::base_config();
	let mut scope_descriptions = HashMap::new();

	scope_descriptions.insert("project_shared".to_string(), "Project notes.".to_string());

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: Some(scope_descriptions.clone()),
		scope_boost_weight: Some(-0.01),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains("context.scope_boost_weight must be zero or greater."),
		"Unexpected error: {err}"
	);

	cfg.context = Some(Context {
		project_descriptions: None,
		scope_descriptions: Some(scope_descriptions),
		scope_boost_weight: Some(1.01),
	});

	let err = elf_config::validate(&cfg).expect_err("Expected context validation error.");

	assert!(
		err.to_string().contains("context.scope_boost_weight must be 1.0 or less."),
		"Unexpected error: {err}"
	);
}
