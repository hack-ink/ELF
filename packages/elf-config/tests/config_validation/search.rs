use std::fs;

use crate::helpers;

#[test]
fn cache_ttl_must_be_positive() {
	let payload = helpers::sample_toml_with_cache(true, 0, 7, true);
	let path = helpers::write_temp_config(payload);
	let result = elf_config::load(&path);

	fs::remove_file(&path).expect("Failed to remove test config.");

	let err = result.expect_err("Expected cache TTL validation error.");

	assert!(
		err.to_string().contains("search.cache.expansion_ttl_days must be greater than zero."),
		"Unexpected error: {err}"
	);
}

#[test]
fn recursive_search_settings_can_be_valid() {
	let mut cfg = helpers::base_config();

	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_depth = 4;
	cfg.search.recursive.max_children_per_node = 12;
	cfg.search.recursive.max_nodes_per_scope = 64;
	cfg.search.recursive.max_total_nodes = 120;

	assert!(elf_config::validate(&cfg).is_ok());
}

#[test]
fn recursive_search_settings_require_valid_depth_bounds() {
	let mut cfg = helpers::base_config();

	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_depth = 0;

	let err =
		elf_config::validate(&cfg).expect_err("Expected recursive max_depth validation error.");

	assert!(
		err.to_string().contains("search.recursive.max_depth must be greater than zero."),
		"Unexpected error: {err}"
	);
}

#[test]
fn recursive_search_settings_require_reasonable_bounds() {
	let mut cfg = helpers::base_config();

	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_children_per_node = 0;

	let err =
		elf_config::validate(&cfg).expect_err("Expected recursive branch factor validation error.");

	assert!(
		err.to_string()
			.contains("search.recursive.max_children_per_node must be greater than zero."),
		"Unexpected error: {err}"
	);

	cfg = helpers::base_config();
	cfg.search.recursive.enabled = true;
	cfg.search.recursive.max_total_nodes = 8;
	cfg.search.recursive.max_nodes_per_scope = 12;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected recursive max_total_nodes lower-bound validation error.");

	assert!(
		err.to_string().contains(
			"search.recursive.max_total_nodes must be at least search.recursive.max_nodes_per_scope."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_facts_per_item_must_be_positive_when_enabled() {
	let mut cfg = helpers::base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_facts_per_item = 0;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected graph_context max_facts_per_item validation error.");

	assert!(
		err.to_string()
			.contains("search.graph_context.max_facts_per_item must be greater than zero."),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_evidence_notes_per_fact_must_be_positive_when_enabled() {
	let mut cfg = helpers::base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_evidence_notes_per_fact = 0;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected graph_context max_evidence_notes_per_fact validation error.");

	assert!(
		err.to_string().contains(
			"search.graph_context.max_evidence_notes_per_fact must be greater than zero."
		),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_facts_per_item_cannot_exceed_hard_limit() {
	let mut cfg = helpers::base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_facts_per_item = 1_001;

	let err = elf_config::validate(&cfg)
		.expect_err("Expected graph_context max_facts_per_item upper-bound validation error.");

	assert!(
		err.to_string().contains("search.graph_context.max_facts_per_item must be 1,000 or less."),
		"Unexpected error: {err}"
	);
}

#[test]
fn graph_context_settings_max_evidence_notes_per_fact_cannot_exceed_hard_limit() {
	let mut cfg = helpers::base_config();

	cfg.search.graph_context.enabled = true;
	cfg.search.graph_context.max_evidence_notes_per_fact = 1_001;

	let err = elf_config::validate(&cfg).expect_err(
		"Expected graph_context max_evidence_notes_per_fact upper-bound validation error.",
	);

	assert!(
		err.to_string()
			.contains("search.graph_context.max_evidence_notes_per_fact must be 1,000 or less."),
		"Unexpected error: {err}"
	);
}
