use sqlx::PgPool;
use uuid::Uuid;

use crate::acceptance::graph_ingestion::tests_helpers::{
	GRAPH_REL_OBJECT, GRAPH_REL_PREDICATE, GRAPH_REL_SUBJECT, TEST_PROJECT, TEST_SCOPE, TEST_TENANT,
};

pub(in crate::acceptance::graph_ingestion) async fn graph_fact_id(pool: &PgPool) -> Uuid {
	sqlx::query_scalar(
		"\
SELECT gf.fact_id
FROM graph_facts gf
JOIN graph_entities ge ON ge.entity_id = gf.subject_entity_id
WHERE ge.canonical_norm = $1
	AND gf.predicate = $2
	AND gf.object_value = $3
	AND gf.tenant_id = $4
	AND gf.project_id = $5
	AND gf.scope = $6",
	)
	.bind(GRAPH_REL_SUBJECT)
	.bind(GRAPH_REL_PREDICATE)
	.bind(GRAPH_REL_OBJECT)
	.bind(TEST_TENANT)
	.bind(TEST_PROJECT)
	.bind(TEST_SCOPE)
	.fetch_one(pool)
	.await
	.expect("Failed to load fact.")
}

pub(in crate::acceptance::graph_ingestion) async fn graph_fact_count(pool: &PgPool) -> i64 {
	sqlx::query_scalar(
		"\
SELECT COUNT(*)
FROM graph_facts gf
JOIN graph_entities ge ON ge.entity_id = gf.subject_entity_id
WHERE ge.canonical_norm = $1
	AND gf.predicate = $2
	AND gf.object_value = $3
	AND gf.tenant_id = $4
	AND gf.project_id = $5
	AND gf.scope = $6",
	)
	.bind(GRAPH_REL_SUBJECT)
	.bind(GRAPH_REL_PREDICATE)
	.bind(GRAPH_REL_OBJECT)
	.bind(TEST_TENANT)
	.bind(TEST_PROJECT)
	.bind(TEST_SCOPE)
	.fetch_one(pool)
	.await
	.expect("Failed to count fact rows.")
}

pub(in crate::acceptance::graph_ingestion) async fn graph_fact_evidence_count(
	pool: &PgPool,
	fact_id: Uuid,
) -> i64 {
	sqlx::query_scalar("SELECT COUNT(*) FROM graph_fact_evidence WHERE fact_id = $1")
		.bind(fact_id)
		.fetch_one(pool)
		.await
		.expect("Failed to load fact evidence.")
}

pub(in crate::acceptance::graph_ingestion) async fn graph_fact_evidence_count_for_note(
	pool: &PgPool,
	fact_id: Uuid,
	note_id: Uuid,
) -> i64 {
	sqlx::query_scalar(
		"SELECT COUNT(*) FROM graph_fact_evidence WHERE fact_id = $1 AND note_id = $2",
	)
	.bind(fact_id)
	.bind(note_id)
	.fetch_one(pool)
	.await
	.expect("Failed to load note evidence.")
}
