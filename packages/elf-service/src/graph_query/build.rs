use crate::{
	graph,
	graph_query::{
		ELF_GRAPH_QUERY_SCHEMA_V1, Error, GraphQueryExplain, GraphQueryFact, GraphQueryFactRow,
		GraphQueryObject, GraphQueryObjectEntity, OffsetDateTime, Result,
	},
};

pub(crate) fn resolve_effective_scopes(
	allowed_scopes: &[String],
	requested_scopes: &[String],
) -> Result<Vec<String>> {
	let allowed = allowed_scopes
		.iter()
		.map(|scope| scope.trim())
		.filter(|scope| !scope.is_empty())
		.collect::<Vec<_>>();

	if allowed.is_empty() {
		return Err(Error::InvalidRequest {
			message: "read_profile resolves to no readable scopes.".to_string(),
		});
	}
	if requested_scopes.is_empty() {
		let mut deduped = Vec::with_capacity(allowed.len());

		for scope in allowed {
			if !deduped.iter().any(|value| value == scope) {
				deduped.push(scope.to_string());
			}
		}

		return Ok(deduped);
	}

	let mut effective = Vec::new();

	for requested_scope in requested_scopes {
		if !allowed.iter().any(|scope| scope == requested_scope) {
			return Err(Error::InvalidRequest {
				message: format!("scope is not readable under read_profile: {}", requested_scope),
			});
		}
		if !effective.iter().any(|scope| scope == requested_scope) {
			effective.push(requested_scope.to_string());
		}
	}

	Ok(effective)
}

pub(crate) fn truncate_graph_query_facts(
	mut facts: Vec<GraphQueryFact>,
	limit: usize,
) -> (Vec<GraphQueryFact>, bool) {
	let truncated = facts.len() > limit;

	if truncated {
		facts.truncate(limit);
	}

	(facts, truncated)
}

pub(crate) fn build_graph_query_explain(
	as_of: OffsetDateTime,
	allowed_scopes: &[String],
	effective_scopes: &[String],
	requested_limit: usize,
	queried_rows: usize,
	returned_rows: usize,
	truncated: bool,
) -> GraphQueryExplain {
	GraphQueryExplain {
		schema: ELF_GRAPH_QUERY_SCHEMA_V1.to_string(),
		as_of,
		requested_limit: requested_limit as u32,
		allowed_scopes: allowed_scopes.to_vec(),
		effective_scopes: effective_scopes.to_vec(),
		queried_rows,
		returned_rows,
		truncated,
	}
}

pub(super) fn graph_query_facts_from_rows(
	rows: Vec<GraphQueryFactRow>,
	read_at: OffsetDateTime,
) -> Vec<GraphQueryFact> {
	rows.into_iter()
		.filter(|row| !row.evidence_note_ids.is_empty())
		.map(|row| {
			let object = if let Some(entity_id) = row.object_entity_id {
				GraphQueryObject {
					entity: Some(GraphQueryObjectEntity {
						entity_id,
						canonical: row.object_canonical.unwrap_or_else(|| "".to_string()),
						kind: row.object_kind,
					}),
					value: None,
				}
			} else {
				GraphQueryObject { entity: None, value: row.object_value }
			};

			GraphQueryFact {
				fact_id: row.fact_id,
				scope: row.scope,
				actor: row.actor,
				predicate: row.predicate,
				predicate_id: row.predicate_id,
				valid_from: row.valid_from,
				valid_to: row.valid_to,
				temporal_status: graph::relation_temporal_status(
					row.valid_from,
					row.valid_to,
					read_at,
				),
				object,
				evidence_note_ids: row.evidence_note_ids,
			}
		})
		.collect()
}
