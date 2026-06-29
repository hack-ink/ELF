use std::collections::HashSet;

use uuid::Uuid;

use crate::{
	ELF_GRAPH_QUERY_SCHEMA_V1, Error, GraphQueryFact, GraphQueryObject, GraphQueryObjectEntity,
	graph::RelationTemporalStatus,
	graph_query::{self, GraphQueryEntityRef, GraphQueryRequest, OffsetDateTime},
};

fn base_request() -> GraphQueryRequest {
	GraphQueryRequest {
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		subject: GraphQueryEntityRef::Surface { surface: "Alice".to_string() },
		predicate: None,
		scopes: None,
		as_of: None,
		limit: Some(10),
		explain: Some(true),
	}
}

#[test]
fn test_validate_graph_query_request_rejects_invalid_fields() {
	let mut request = base_request();

	request.subject = GraphQueryEntityRef::Surface { surface: "   ".to_string() };

	let err = graph_query::validate_graph_query_request(request)
		.expect_err("invalid subject should fail");

	assert!(matches!(err, Error::InvalidRequest { .. }), "expected invalid request error");
}

#[test]
fn test_truncate_graph_query_facts_and_explain_shaping() {
	let facts = vec![
		GraphQueryFact {
			fact_id: Uuid::from_u128(1),
			scope: "project_shared".to_string(),
			actor: "agent1".to_string(),
			predicate: "knows".to_string(),
			predicate_id: None,
			valid_from: OffsetDateTime::from_unix_timestamp(1).expect("valid timestamp"),
			valid_to: None,
			temporal_status: RelationTemporalStatus::Current,
			object: GraphQueryObject {
				entity: Some(GraphQueryObjectEntity {
					entity_id: Uuid::from_u128(100),
					canonical: "Bob".to_string(),
					kind: Some("person".to_string()),
				}),
				value: None,
			},
			evidence_note_ids: vec![],
		},
		GraphQueryFact {
			fact_id: Uuid::from_u128(2),
			scope: "project_shared".to_string(),
			actor: "agent1".to_string(),
			predicate: "likes".to_string(),
			predicate_id: None,
			valid_from: OffsetDateTime::from_unix_timestamp(2).expect("valid timestamp"),
			valid_to: None,
			temporal_status: RelationTemporalStatus::Current,
			object: GraphQueryObject {
				entity: Some(GraphQueryObjectEntity {
					entity_id: Uuid::from_u128(101),
					canonical: "Carol".to_string(),
					kind: Some("person".to_string()),
				}),
				value: None,
			},
			evidence_note_ids: vec![],
		},
		GraphQueryFact {
			fact_id: Uuid::from_u128(3),
			scope: "project_shared".to_string(),
			actor: "agent2".to_string(),
			predicate: "located_in".to_string(),
			predicate_id: None,
			valid_from: OffsetDateTime::from_unix_timestamp(3).expect("valid timestamp"),
			valid_to: None,
			temporal_status: RelationTemporalStatus::Current,
			object: GraphQueryObject { entity: None, value: Some("office".to_string()) },
			evidence_note_ids: vec![],
		},
	];
	let (trimmed, truncated) = graph_query::truncate_graph_query_facts(facts, 2);

	assert!(truncated);
	assert_eq!(trimmed.len(), 2);

	let explain = graph_query::build_graph_query_explain(
		OffsetDateTime::from_unix_timestamp(4).expect("valid timestamp"),
		&["private_plus_project".to_string()],
		&["private_plus_project".to_string()],
		2,
		3,
		trimmed.len(),
		truncated,
	);

	assert_eq!(explain.queried_rows, 3);
	assert_eq!(explain.returned_rows, 2);
	assert!(explain.truncated);
	assert_eq!(explain.schema, ELF_GRAPH_QUERY_SCHEMA_V1);
}

#[test]
fn test_resolve_effective_scopes_validates_requested_scopes() {
	let allowed =
		vec!["agent_private".to_string(), "project_shared".to_string(), "org_shared".to_string()];
	let requested = vec!["project_shared".to_string(), "project_shared".to_string()];
	let resolved =
		graph_query::resolve_effective_scopes(&allowed, &requested).expect("valid scopes");
	let deduped: HashSet<_> = resolved.iter().collect();

	assert_eq!(resolved, vec!["project_shared".to_string()]);
	assert_eq!(deduped.len(), 1);
}

#[test]
fn graph_query_rows_without_readable_evidence_are_suppressed() {
	let read_at = OffsetDateTime::from_unix_timestamp(30).expect("valid timestamp");
	let rows = vec![
		super::GraphQueryFactRow {
			fact_id: Uuid::from_u128(1),
			scope: "agent_private".to_string(),
			actor: "agent".to_string(),
			predicate: "works at".to_string(),
			predicate_id: None,
			object_entity_id: None,
			object_canonical: None,
			object_kind: None,
			object_value: Some("Deleted Source Inc.".to_string()),
			valid_from: OffsetDateTime::from_unix_timestamp(10).expect("valid timestamp"),
			valid_to: None,
			evidence_note_ids: vec![],
		},
		super::GraphQueryFactRow {
			fact_id: Uuid::from_u128(2),
			scope: "agent_private".to_string(),
			actor: "agent".to_string(),
			predicate: "works at".to_string(),
			predicate_id: None,
			object_entity_id: None,
			object_canonical: None,
			object_kind: None,
			object_value: Some("Active Source Inc.".to_string()),
			valid_from: OffsetDateTime::from_unix_timestamp(20).expect("valid timestamp"),
			valid_to: None,
			evidence_note_ids: vec![Uuid::from_u128(200)],
		},
	];
	let facts = super::graph_query_facts_from_rows(rows, read_at);

	assert_eq!(facts.len(), 1);
	assert_eq!(facts[0].fact_id, Uuid::from_u128(2));
	assert_eq!(facts[0].object.value.as_deref(), Some("Active Source Inc."));
	assert_eq!(facts[0].evidence_note_ids, vec![Uuid::from_u128(200)]);
}
