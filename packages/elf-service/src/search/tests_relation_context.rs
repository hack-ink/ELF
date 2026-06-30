use crate::{
	ElfService,
	search::{OffsetDateTime, Uuid},
};

#[test]
fn relation_context_rows_without_evidence_are_suppressed() {
	let now = OffsetDateTime::from_unix_timestamp(100).expect("valid timestamp");
	let note_id = Uuid::from_u128(1);
	let contexts =
		ElfService::group_relation_context_rows(vec![crate::search::SearchRelationContextRow {
			note_id,
			fact_id: Uuid::from_u128(2),
			scope: "project_shared".to_string(),
			subject_canonical: Some("Alice".to_string()),
			subject_kind: Some("person".to_string()),
			predicate: "prefers".to_string(),
			object_entity_id: None,
			object_canonical: None,
			object_kind: None,
			object_value: Some("source-bound recall".to_string()),
			valid_from: now,
			valid_to: None,
			is_current: true,
			evidence_note_ids: Vec::new(),
		}]);

	assert!(!contexts.contains_key(&note_id));
}

#[test]
fn relation_context_sql_enforces_shared_grant_keys() {
	assert!(
		crate::search::RELATION_CONTEXT_SQL
			.contains("concat(gf.scope, ':', gf.agent_id) = ANY($10::text[])")
	);
	assert!(
		crate::search::RELATION_CONTEXT_SQL.contains(
			"concat(evidence_note.scope, ':', evidence_note.agent_id) = ANY($10::text[])"
		)
	);
}
