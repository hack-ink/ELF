use crate::graph_query::{
	Error, GraphEntity, GraphQueryEntityRef, GraphQueryPredicateRef, PgConnection,
	ResolvedGraphQueryPredicate, ResolvedGraphQuerySubject, Result, graph,
};

pub(super) async fn resolve_subject(
	conn: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	subject: GraphQueryEntityRef,
) -> Result<ResolvedGraphQuerySubject> {
	match subject {
		GraphQueryEntityRef::EntityId { entity_id } => {
			let row = sqlx::query_as::<_, GraphEntity>(
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
	AND entity_id = $3",
			)
			.bind(tenant_id)
			.bind(project_id)
			.bind(entity_id)
			.fetch_optional(conn)
			.await?;
			let Some(row) = row else {
				return Err(Error::NotFound {
					message: format!("graph entity not found for subject entity_id={entity_id}"),
				});
			};

			Ok(ResolvedGraphQuerySubject {
				entity_id: row.entity_id,
				canonical: row.canonical,
				kind: row.kind,
			})
		},
		GraphQueryEntityRef::Surface { surface } => {
			let Some(row) =
				graph::resolve_entity_by_surface(conn, tenant_id, project_id, &surface).await?
			else {
				return Err(Error::NotFound {
					message: format!("graph entity not found for subject surface={surface}"),
				});
			};

			Ok(ResolvedGraphQuerySubject {
				entity_id: row.entity_id,
				canonical: row.canonical,
				kind: row.kind,
			})
		},
	}
}

pub(super) async fn resolve_predicate(
	conn: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	predicate: Option<GraphQueryPredicateRef>,
) -> Result<Option<ResolvedGraphQueryPredicate>> {
	let Some(predicate) = predicate else {
		return Ok(None);
	};

	match predicate {
		GraphQueryPredicateRef::PredicateId { predicate_id } => {
			let row = graph::get_predicate_by_id(conn, predicate_id).await?;
			let Some(row) = row else {
				return Err(Error::NotFound {
					message: format!("graph predicate not found: {predicate_id}"),
				});
			};

			Ok(Some(ResolvedGraphQueryPredicate { id: row.predicate_id, canonical: row.canonical }))
		},
		GraphQueryPredicateRef::Surface { surface } => {
			let Some(row) =
				graph::resolve_predicate_no_register(conn, tenant_id, project_id, &surface).await?
			else {
				return Err(Error::NotFound {
					message: format!("graph predicate not found for surface={surface}"),
				});
			};

			Ok(Some(ResolvedGraphQueryPredicate { id: row.predicate_id, canonical: row.canonical }))
		},
	}
}
