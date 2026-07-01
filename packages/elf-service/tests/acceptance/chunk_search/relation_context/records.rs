use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) async fn insert_graph_entity<'e, E>(
	executor: E,
	entity_id: Uuid,
	canonical: &str,
	kind: Option<&str>,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_entities (
	entity_id,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	kind
)
VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(entity_id)
	.bind("t")
	.bind("p")
	.bind(canonical)
	.bind(canonical.to_lowercase())
	.bind(kind)
	.execute(executor)
	.await
	.expect("Failed to insert graph entity.");
}

pub(super) async fn insert_graph_predicate<'e, E>(executor: E, predicate_id: Uuid, canonical: &str)
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_predicates (
	predicate_id,
	scope_key,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	cardinality,
	status
)
VALUES ($1, $2, $3, $4, $5, $6, 'single', 'active')",
	)
	.bind(predicate_id)
	.bind("__project__:p")
	.bind("t")
	.bind("p")
	.bind(canonical)
	.bind(canonical.to_lowercase())
	.execute(executor)
	.await
	.expect("Failed to insert graph predicate.");
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_graph_fact<'e, E>(
	executor: E,
	fact_id: Uuid,
	subject_entity_id: Uuid,
	predicate: &str,
	predicate_id: Uuid,
	object_value: &str,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_facts (
	fact_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	subject_entity_id,
	predicate,
	predicate_id,
	object_entity_id,
	object_value,
	valid_from,
	valid_to
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, $10, $11)",
	)
	.bind(fact_id)
	.bind("t")
	.bind("p")
	.bind("a")
	.bind("agent_private")
	.bind(subject_entity_id)
	.bind(predicate)
	.bind(predicate_id)
	.bind(object_value)
	.bind(valid_from)
	.bind(valid_to)
	.execute(executor)
	.await
	.expect("Failed to insert graph fact.");
}

pub(super) async fn insert_graph_fact_evidence<'e, E>(
	executor: E,
	fact_id: Uuid,
	note_id: Uuid,
	created_at: OffsetDateTime,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO graph_fact_evidence (evidence_id, fact_id, note_id, created_at)
VALUES ($1, $2, $3, $4)",
	)
	.bind(Uuid::new_v4())
	.bind(fact_id)
	.bind(note_id)
	.bind(created_at)
	.execute(executor)
	.await
	.expect("Failed to insert graph fact evidence.");
}
