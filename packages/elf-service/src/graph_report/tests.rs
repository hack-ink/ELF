use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	RelationTemporalStatus,
	graph_report::{self, GraphReportFactRow},
};

fn ts(value: i64) -> OffsetDateTime {
	OffsetDateTime::from_unix_timestamp(value).expect("valid timestamp")
}

fn row(
	raw_id: u128,
	object_value: &str,
	valid_from: i64,
	valid_to: Option<i64>,
	predicate_status: &str,
	cardinality: &str,
	superseded_by: Vec<Uuid>,
) -> GraphReportFactRow {
	GraphReportFactRow {
		fact_id: Uuid::from_u128(raw_id),
		scope: "agent_private".to_string(),
		actor: "agent".to_string(),
		predicate: "works at".to_string(),
		predicate_id: Some(Uuid::from_u128(999)),
		predicate_status: Some(predicate_status.to_string()),
		predicate_cardinality: Some(cardinality.to_string()),
		object_entity_id: None,
		object_canonical: None,
		object_kind: None,
		object_value: Some(object_value.to_string()),
		valid_from: ts(valid_from),
		valid_to: valid_to.map(ts),
		evidence_note_ids: vec![Uuid::from_u128(raw_id + 10_000)],
		superseded_by_fact_ids: superseded_by,
		supersedes_fact_ids: vec![],
	}
}

#[test]
fn graph_report_classifies_temporal_source_and_supersession_markers() {
	let replacement_id = Uuid::from_u128(2);
	let facts = graph_report::build_report_facts(
		vec![
			row(1, "Initech", 10, Some(20), "active", "single", vec![replacement_id]),
			row(2, "Globex", 20, None, "active", "single", vec![]),
			row(3, "Umbrella", 30, None, "pending", "single", vec![]),
		],
		ts(25),
	);
	let summary = graph_report::summarize_report_facts(&facts);

	assert_eq!(summary.fact_count, 3);
	assert_eq!(summary.current_count, 1);
	assert_eq!(summary.historical_count, 1);
	assert_eq!(summary.future_count, 1);
	assert_eq!(summary.sourced_count, 3);
	assert_eq!(summary.inferred_count, 1);
	assert_eq!(summary.stale_count, 1);
	assert_eq!(summary.superseded_count, 1);
	assert_eq!(summary.evidence_link_count, 3);
	assert_eq!(facts[0].temporal_status, RelationTemporalStatus::Historical);
	assert!(facts[0].status_markers.iter().any(|marker| marker == "superseded"));
	assert!(facts[2].status_markers.iter().any(|marker| marker == "inferred"));
}

#[test]
fn graph_report_suppresses_facts_without_readable_evidence() {
	let mut deleted_source = row(1, "Deleted Source Inc.", 10, None, "active", "single", vec![]);

	deleted_source.evidence_note_ids = vec![];

	let facts = graph_report::build_report_facts(
		vec![deleted_source, row(2, "Active Source Inc.", 20, None, "active", "single", vec![])],
		ts(25),
	);

	assert_eq!(facts.len(), 1);
	assert_eq!(facts[0].fact_id, Uuid::from_u128(2));
	assert_eq!(facts[0].object.value.as_deref(), Some("Active Source Inc."));
}

#[test]
fn graph_topic_map_preserves_fact_edges_and_source_markers() {
	let subject = super::ResolvedGraphReportSubject {
		entity_id: Uuid::from_u128(42),
		canonical: "Alice".to_string(),
		kind: Some("person".to_string()),
	};
	let facts = graph_report::build_report_facts(
		vec![row(1, "Globex", 20, None, "active", "single", vec![])],
		ts(25),
	);
	let topic_map = graph_report::build_topic_map(&subject, &facts);

	assert_eq!(topic_map.nodes.len(), 2);
	assert_eq!(topic_map.edges.len(), 1);
	assert_eq!(topic_map.edges[0].predicate, "works at");
	assert_eq!(topic_map.edges[0].temporal_status, RelationTemporalStatus::Current);
	assert!(topic_map.edges[0].status_markers.iter().any(|marker| marker == "sourced"));
}
