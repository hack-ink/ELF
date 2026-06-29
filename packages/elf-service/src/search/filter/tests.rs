use std::collections::HashMap;

use serde_json::{Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::search::{ChunkCandidate, NoteMeta};

use super::{
	SearchFilter,
	parser::{MAX_FILTER_NODES, MAX_IN_LIST_ITEMS, MAX_STRING_BYTES, SEARCH_FILTER_EXPR_SCHEMA_V1},
};

fn note_meta() -> NoteMeta {
	NoteMeta {
		note_id: Uuid::new_v4(),
		note_type: "fact".to_string(),
		key: Some("foo".to_string()),
		scope: "project_shared".to_string(),
		agent_id: "agent-a".to_string(),
		importance: 0.9,
		confidence: 0.8,
		updated_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("timestamp"),
		expires_at: None,
		source_ref: Value::Object(Map::new()),
		embedding_version: "provider:model:1".to_string(),
		hit_count: 4,
		last_hit_at: None,
	}
}

#[test]
fn parse_requires_known_schema() {
	let raw = serde_json::json!({ "schema": "bad", "expr": { "op": "eq", "field": "scope", "value": "project_shared" } });

	assert!(SearchFilter::parse(&raw).is_err());
}

#[test]
fn parse_and_validate_depth_limit() {
	let mut expr = serde_json::json!({ "op": "eq", "field": "scope", "value": "project_shared" });

	for _ in 0..9 {
		expr = serde_json::json!({ "op": "not", "expr": expr });
	}

	let raw = serde_json::json!({ "schema": SEARCH_FILTER_EXPR_SCHEMA_V1, "expr": expr });

	assert!(SearchFilter::parse(&raw).is_err());
}

#[test]
fn parse_and_validate_node_limit() {
	let leaf = serde_json::json!({ "op": "eq", "field": "scope", "value": "project_shared" });
	let mut args = Vec::with_capacity(MAX_FILTER_NODES);

	for _ in 0..(MAX_FILTER_NODES - 1) {
		args.push(leaf.clone());
	}

	let expr = serde_json::json!({ "op": "and", "args": args });
	let raw = serde_json::json!({ "schema": SEARCH_FILTER_EXPR_SCHEMA_V1, "expr": expr });

	assert!(SearchFilter::parse(&raw).is_ok());

	let expr = serde_json::json!({ "op": "and", "args": [expr, leaf] });
	let raw = serde_json::json!({ "schema": SEARCH_FILTER_EXPR_SCHEMA_V1, "expr": expr });

	assert!(
		SearchFilter::parse(&raw).is_err(),
		"expected parse failure when node count is greater than limit"
	);
}

#[test]
fn parse_in_list_limit() {
	let values = (0_i32..=MAX_IN_LIST_ITEMS as i32)
		.map(|value| serde_json::json!(value))
		.collect::<Vec<_>>();
	let raw = serde_json::json!({
		"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
		"expr": {
			"op": "in",
			"field": "importance",
			"value": values,
		},
	});

	assert!(SearchFilter::parse(&raw).is_err());
}

#[test]
fn parse_rejects_unknown_field_with_json_path() {
	let raw = serde_json::json!({
		"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
		"expr": { "op": "eq", "field": "bad_field", "value": "project_shared" },
	});
	let err = SearchFilter::parse(&raw).expect_err("expected unknown field error");

	assert!(err.to_string().contains("$.filter.expr"));
	assert!(err.to_string().contains("not in allowlist"));
}

#[test]
fn parse_rejects_invalid_value_type_with_json_path() {
	let raw = serde_json::json!({
		"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
		"expr": { "op": "eq", "field": "importance", "value": "wrong" },
	});
	let err = SearchFilter::parse(&raw).expect_err("expected invalid value type error");

	assert!(err.to_string().contains("$.filter.expr.value"));
}

#[test]
fn parse_rejects_oversize_string_with_json_path() {
	let value = "x".repeat(MAX_STRING_BYTES + 1);
	let raw = serde_json::json!({
		"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
		"expr": { "op": "eq", "field": "scope", "value": value },
	});
	let err = SearchFilter::parse(&raw).expect_err("expected string too long error");

	assert!(err.to_string().contains("$.filter.expr.value"));
}

#[test]
fn eval_filters_note_metadata() {
	let raw = serde_json::json!({
		"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
		"expr": {
			"op": "and",
			"args": [
				{ "op": "eq", "field": "scope", "value": "project_shared" },
				{ "op": "gte", "field": "importance", "value": 0.5 },
			],
		},
	});
	let filter = SearchFilter::parse(&raw).expect("valid filter");
	let meta = note_meta();
	let note_meta = HashMap::from([(meta.note_id, meta)]);
	let candidate = ChunkCandidate {
		note_id: Uuid::new_v4(),
		chunk_id: Uuid::new_v4(),
		chunk_index: 0,
		retrieval_rank: 1,
		retrieval_score: None,
		scope: Some("project_shared".to_string()),
		updated_at: None,
		embedding_version: None,
	};
	let (result, impact) = filter.eval(vec![candidate], &note_meta, 10, 12);

	assert_eq!(result.len(), 0);
	assert_eq!(impact.requested_candidate_k, 10);
	assert_eq!(impact.effective_candidate_k, 12);
}

#[test]
fn filter_impact_lists_top_drop_reasons_deterministically() {
	let filter = SearchFilter::parse(&serde_json::json!({
		"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
		"expr": { "op": "eq", "field": "scope", "value": "project_shared" },
	}))
	.expect("valid filter");
	let first = Uuid::new_v4();
	let second = Uuid::new_v4();
	let third = Uuid::new_v4();
	let mut note_meta = HashMap::new();

	note_meta.insert(
		first,
		NoteMeta {
			note_id: first,
			note_type: "fact".to_string(),
			key: Some("k1".to_string()),
			scope: "agent_private".to_string(),
			agent_id: "a".to_string(),
			importance: 0.9,
			confidence: 0.9,
			updated_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("timestamp"),
			expires_at: None,
			source_ref: Value::Object(Map::new()),
			embedding_version: "provider:model:1".to_string(),
			hit_count: 0,
			last_hit_at: None,
		},
	);
	note_meta.insert(
		second,
		NoteMeta {
			note_id: second,
			note_type: "fact".to_string(),
			key: Some("k2".to_string()),
			scope: "agent_private".to_string(),
			agent_id: "a".to_string(),
			importance: 0.9,
			confidence: 0.9,
			updated_at: OffsetDateTime::from_unix_timestamp(1_700_000_001).expect("timestamp"),
			expires_at: None,
			source_ref: Value::Object(Map::new()),
			embedding_version: "provider:model:1".to_string(),
			hit_count: 0,
			last_hit_at: None,
		},
	);

	let candidates = vec![
		ChunkCandidate {
			note_id: first,
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			retrieval_rank: 1,
			retrieval_score: None,
			scope: None,
			updated_at: None,
			embedding_version: None,
		},
		ChunkCandidate {
			note_id: second,
			chunk_id: Uuid::new_v4(),
			chunk_index: 1,
			retrieval_rank: 2,
			retrieval_score: None,
			scope: None,
			updated_at: None,
			embedding_version: None,
		},
		ChunkCandidate {
			note_id: third,
			chunk_id: Uuid::new_v4(),
			chunk_index: 2,
			retrieval_rank: 3,
			retrieval_score: None,
			scope: None,
			updated_at: None,
			embedding_version: None,
		},
	];
	let (_, impact) = filter.eval(candidates, &note_meta, 10, 20);

	assert_eq!(impact.candidate_count_pre, 3);
	assert_eq!(impact.candidate_count_post, 0);
	assert_eq!(impact.dropped_total, 3);
	assert_eq!(impact.top_drop_reasons.len(), 2);
	assert_eq!(impact.top_drop_reasons[0].reason, "eq:scope");
	assert_eq!(impact.top_drop_reasons[0].count, 2);
	assert_eq!(impact.top_drop_reasons[1].reason, "note_meta_missing");
	assert_eq!(impact.top_drop_reasons[1].count, 1);
}
