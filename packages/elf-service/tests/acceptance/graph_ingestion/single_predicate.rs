use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::acceptance::graph_ingestion::tests_helpers::{
	self, GRAPH_REL_SUBJECT, TEST_PROJECT, TEST_SCOPE, TEST_TENANT,
};
use elf_service::{
	GraphQueryEntityRef, GraphQueryPredicateRef, GraphQueryRequest, RelationTemporalStatus,
};

#[derive(Debug, FromRow)]
struct GraphFactRow {
	fact_id: Uuid,
	predicate_id: Option<Uuid>,
	object_value: Option<String>,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
}

fn works_at_graph_query_request(as_of: OffsetDateTime) -> GraphQueryRequest {
	GraphQueryRequest {
		tenant_id: TEST_TENANT.to_string(),
		project_id: TEST_PROJECT.to_string(),
		agent_id: "a".to_string(),
		read_profile: "private_only".to_string(),
		subject: GraphQueryEntityRef::Surface { surface: "Alice".to_string() },
		predicate: Some(GraphQueryPredicateRef::Surface { surface: "works at".to_string() }),
		scopes: Some(vec![TEST_SCOPE.to_string()]),
		as_of: Some(as_of),
		limit: Some(10),
		explain: Some(true),
	}
}

async fn graph_fact_row(pool: &PgPool, predicate: &str, object_value: &str) -> GraphFactRow {
	sqlx::query_as::<_, GraphFactRow>(
		"\
SELECT
	gf.fact_id,
	gf.predicate_id,
	gf.object_value,
	gf.valid_from,
	gf.valid_to
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
	.bind(predicate)
	.bind(object_value)
	.bind(TEST_TENANT)
	.bind(TEST_PROJECT)
	.bind(TEST_SCOPE)
	.fetch_one(pool)
	.await
	.expect("Failed to load fact row.")
}

async fn activate_single_predicate(pool: &PgPool, predicate_id: Uuid) {
	sqlx::query(
		"\
UPDATE graph_predicates
SET status = 'active', cardinality = 'single', updated_at = now()
WHERE predicate_id = $1",
	)
	.bind(predicate_id)
	.execute(pool)
	.await
	.expect("Failed to activate predicate.");
}

async fn active_object_value_at(
	pool: &PgPool,
	predicate_id: Uuid,
	at: OffsetDateTime,
) -> Option<String> {
	sqlx::query_scalar(
		"\
SELECT gf.object_value
FROM graph_facts gf
JOIN graph_entities ge ON ge.entity_id = gf.subject_entity_id
WHERE ge.canonical_norm = $1
	AND gf.tenant_id = $2
	AND gf.project_id = $3
	AND gf.scope = $4
	AND gf.predicate_id = $5
	AND gf.valid_from <= $6
	AND (gf.valid_to IS NULL OR gf.valid_to > $6)
LIMIT 1",
	)
	.bind(GRAPH_REL_SUBJECT)
	.bind(TEST_TENANT)
	.bind(TEST_PROJECT)
	.bind(TEST_SCOPE)
	.bind(predicate_id)
	.bind(at)
	.fetch_one(pool)
	.await
	.expect("Failed to load active fact object_value.")
}

async fn active_fact_count_at(pool: &PgPool, predicate_id: Uuid, at: OffsetDateTime) -> i64 {
	sqlx::query_scalar(
		"\
SELECT COUNT(*)
FROM graph_facts gf
JOIN graph_entities ge ON ge.entity_id = gf.subject_entity_id
WHERE ge.canonical_norm = $1
	AND gf.tenant_id = $2
	AND gf.project_id = $3
	AND gf.scope = $4
	AND gf.predicate_id = $5
	AND gf.valid_from <= $6
	AND (gf.valid_to IS NULL OR gf.valid_to > $6)",
	)
	.bind(GRAPH_REL_SUBJECT)
	.bind(TEST_TENANT)
	.bind(TEST_PROJECT)
	.bind(TEST_SCOPE)
	.bind(predicate_id)
	.bind(at)
	.fetch_one(pool)
	.await
	.expect("Failed to count active facts.")
}

async fn supersession_count(
	pool: &PgPool,
	from_fact_id: Uuid,
	to_fact_id: Uuid,
	note_id: Uuid,
) -> i64 {
	sqlx::query_scalar(
		"\
SELECT COUNT(*)
FROM graph_fact_supersessions
WHERE from_fact_id = $1
	AND to_fact_id = $2
	AND note_id = $3",
	)
	.bind(from_fact_id)
	.bind(to_fact_id)
	.bind(note_id)
	.fetch_one(pool)
	.await
	.expect("Failed to count supersessions.")
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn add_note_single_predicate_supersedes_conflicting_fact() {
	let Some(test_db) =
		tests_helpers::build_test_db("add_note_single_predicate_supersedes_conflicting_fact").await
	else {
		return;
	};
	let service = tests_helpers::build_stub_service(&test_db).await;

	tests_helpers::reset_service_db(&service).await;

	let old_note_id = tests_helpers::add_fact_note(
		&service,
		"employment-a",
		"Alice works at Initech.",
		"works at",
		"Initech",
	)
	.await;
	let fact_a = graph_fact_row(&service.db.pool, "works at", "Initech").await;
	let predicate_id = fact_a.predicate_id.expect("Expected predicate_id.");

	activate_single_predicate(&service.db.pool, predicate_id).await;

	tokio::time::sleep(std::time::Duration::from_millis(1)).await;

	let note_id = tests_helpers::add_fact_note(
		&service,
		"employment-b",
		"Alice works at Globex.",
		"works at",
		"Globex",
	)
	.await;
	let fact_a = graph_fact_row(&service.db.pool, "works at", "Initech").await;
	let fact_b = graph_fact_row(&service.db.pool, "works at", "Globex").await;

	assert_eq!(fact_a.predicate_id, Some(predicate_id));
	assert_eq!(fact_b.predicate_id, Some(predicate_id));
	assert_eq!(fact_a.object_value.as_deref(), Some("Initech"));
	assert_eq!(fact_b.object_value.as_deref(), Some("Globex"));
	assert_eq!(fact_a.valid_to, Some(fact_b.valid_from));
	assert!(fact_b.valid_to.is_none());

	let t_before = fact_b.valid_from - time::Duration::microseconds(1);
	let active_before = active_object_value_at(&service.db.pool, predicate_id, t_before).await;

	assert_eq!(active_before.as_deref(), Some("Initech"));

	let t_after = fact_b.valid_from + time::Duration::microseconds(1);
	let active_after = active_object_value_at(&service.db.pool, predicate_id, t_after).await;

	assert_eq!(active_after.as_deref(), Some("Globex"));

	let historical_replay = service
		.graph_query(works_at_graph_query_request(t_before))
		.await
		.expect("historical graph query failed.");

	assert_eq!(historical_replay.facts.len(), 1);
	assert_eq!(historical_replay.facts[0].object.value.as_deref(), Some("Initech"));
	assert_eq!(historical_replay.facts[0].valid_to, Some(fact_b.valid_from));
	assert_eq!(historical_replay.facts[0].temporal_status, RelationTemporalStatus::Historical);
	assert_eq!(historical_replay.facts[0].evidence_note_ids, vec![old_note_id]);

	let current_readback = service
		.graph_query(works_at_graph_query_request(t_after))
		.await
		.expect("current graph query failed.");

	assert_eq!(current_readback.facts.len(), 1);
	assert_eq!(current_readback.facts[0].object.value.as_deref(), Some("Globex"));
	assert_eq!(current_readback.facts[0].temporal_status, RelationTemporalStatus::Current);
	assert_eq!(current_readback.facts[0].evidence_note_ids, vec![note_id]);

	let supersession_count =
		supersession_count(&service.db.pool, fact_a.fact_id, fact_b.fact_id, note_id).await;

	assert_eq!(supersession_count, 1);

	let now = OffsetDateTime::now_utc();
	let active_count = active_fact_count_at(&service.db.pool, predicate_id, now).await;

	assert_eq!(active_count, 1);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
