use serde_json::Value;

use crate::search::{self, ranking};
use elf_config::SearchDynamic;

#[test]
fn dense_embedding_input_includes_project_context_suffix() {
	let input =
		ranking::build_dense_embedding_input("Find payments code.", Some("This is a billing API."));

	assert!(input.starts_with("Find payments code.\n\nProject context:\n"));
	assert!(input.contains("This is a billing API."));
}

#[test]
fn dense_embedding_input_skips_empty_project_context() {
	let input = ranking::build_dense_embedding_input("Find payments code.", Some("   "));

	assert_eq!(input, "Find payments code.");
}

#[test]
fn normalize_queries_includes_original_and_dedupes() {
	let queries = vec!["alpha".to_string(), "beta".to_string(), "alpha".to_string()];
	let normalized = ranking::normalize_queries(queries, "alpha", true, 4);

	assert_eq!(normalized, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn normalize_queries_respects_max_queries() {
	let queries =
		vec!["one".to_string(), "two".to_string(), "three".to_string(), "four".to_string()];
	let normalized = ranking::normalize_queries(queries, "zero", true, 3);

	assert_eq!(normalized.len(), 3);
}

#[test]
fn dynamic_trigger_checks_candidates_and_score() {
	let cfg = SearchDynamic { min_candidates: 10, min_top_score: 0.2 };

	assert!(ranking::should_expand_dynamic(5, 0.9, &cfg));
	assert!(ranking::should_expand_dynamic(20, 0.1, &cfg));
	assert!(!ranking::should_expand_dynamic(20, 0.9, &cfg));
}

#[test]
fn rank_normalize_maps_rank_to_unit_interval() {
	assert!((ranking::rank_normalize(1, 1) - 1.0).abs() < 1e-6);
	assert!((ranking::rank_normalize(1, 5) - 1.0).abs() < 1e-6);
	assert!((ranking::rank_normalize(3, 5) - 0.5).abs() < 1e-6);
	assert!((ranking::rank_normalize(5, 5) - 0.0).abs() < 1e-6);
	assert!((ranking::rank_normalize(0, 5) - 0.0).abs() < 1e-6);
}

#[test]
fn build_trace_audit_includes_token_id_when_present() {
	let audit = search::build_trace_audit("agent-a", Some("tok-123"));

	assert_eq!(audit.get("actor_id"), Some(&Value::from("agent-a")));
	assert_eq!(audit.get("token_id"), Some(&Value::from("tok-123")));
}

#[test]
fn build_trace_audit_omits_token_id_when_empty() {
	let audit = search::build_trace_audit("agent-a", Some("   "));

	assert_eq!(audit.get("actor_id"), Some(&Value::from("agent-a")));
	assert!(audit.get("token_id").is_none());
}
