use crate::graph::{self, Error, GraphEntity, PgConnection, Result, Uuid};

/// Resolves an entity surface against canonical names and aliases within one tenant/project.
pub async fn resolve_entity_by_surface(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	entity_surface: &str,
) -> Result<Option<GraphEntity>> {
	let entity_surface = entity_surface.trim();

	if entity_surface.is_empty() {
		return Err(Error::InvalidArgument(
			"graph entity is required; entity_surface must not be empty".to_string(),
		));
	}

	let canonical_norm = graph::normalize_entity_name(entity_surface);
	let canonical = sqlx::query_as::<_, GraphEntity>(
		"\
SELECT
	entity_id,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	kind,
	created_at,
	updated_at
FROM graph_entities
WHERE tenant_id = $1
	AND project_id = $2
	AND canonical_norm = $3",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(&canonical_norm)
	.fetch_optional(&mut *executor)
	.await?;

	if let Some(entity) = canonical {
		return Ok(Some(entity));
	}

	let alias_matches = sqlx::query_as::<_, GraphEntity>(
		"\
SELECT
	ge.entity_id,
	ge.tenant_id,
	ge.project_id,
	ge.canonical,
	ge.canonical_norm,
	ge.kind,
	ge.created_at,
	ge.updated_at
FROM graph_entity_aliases gea
JOIN graph_entities ge ON ge.entity_id = gea.entity_id
WHERE ge.tenant_id = $1
	AND ge.project_id = $2
	AND gea.alias_norm = $3",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(&canonical_norm)
	.fetch_all(&mut *executor)
	.await?;

	if alias_matches.len() == 1 {
		return Ok(alias_matches.into_iter().next());
	}
	if alias_matches.len() > 1 {
		let candidates = alias_matches
			.iter()
			.map(|entity| entity.entity_id.to_string())
			.collect::<Vec<_>>()
			.join(", ");

		return Err(Error::Conflict(format!(
			"graph entity surface is ambiguous; entity_surface={entity_surface} alias_norm={canonical_norm} candidates=[{candidates}]"
		)));
	}

	Ok(None)
}

/// Upserts an entity by normalized canonical surface and returns its identifier.
pub async fn upsert_entity(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	canonical: &str,
	kind: Option<&str>,
) -> Result<Uuid> {
	let canonical_norm = graph::normalize_entity_name(canonical);
	let row: (Uuid,) = sqlx::query_as(
		"\
INSERT INTO graph_entities (
	entity_id,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	kind,
	created_at,
	updated_at
)
VALUES (
	$1, $2, $3, $4, $5, $6, now(), now()
)
ON CONFLICT (tenant_id, project_id, canonical_norm)
DO UPDATE
SET
	canonical = EXCLUDED.canonical,
	kind = COALESCE(EXCLUDED.kind, graph_entities.kind),
	updated_at = now()
RETURNING entity_id",
	)
	.bind(Uuid::new_v4())
	.bind(tenant_id)
	.bind(project_id)
	.bind(canonical)
	.bind(&canonical_norm)
	.bind(kind)
	.fetch_one(executor)
	.await?;

	Ok(row.0)
}

/// Upserts an alias for an existing entity.
pub async fn upsert_entity_alias(
	executor: &mut PgConnection,
	entity_id: Uuid,
	alias: &str,
) -> Result<()> {
	let alias_norm = graph::normalize_entity_name(alias);

	sqlx::query(
		"\
INSERT INTO graph_entity_aliases (
	alias_id,
	entity_id,
	alias,
	alias_norm,
	created_at
)
VALUES ($1, $2, $3, $4, now())
ON CONFLICT (entity_id, alias_norm)
DO UPDATE SET alias = EXCLUDED.alias",
	)
	.bind(Uuid::new_v4())
	.bind(entity_id)
	.bind(alias)
	.bind(&alias_norm)
	.execute(executor)
	.await?;

	Ok(())
}
