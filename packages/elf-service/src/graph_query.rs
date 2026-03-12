//! Structured graph query APIs.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result, access, search};
use elf_storage::{graph, models::GraphEntity};

/// Schema identifier for graph-query responses.
pub const ELF_GRAPH_QUERY_SCHEMA_V1: &str = "elf.graph_query/v1";

const DEFAULT_GRAPH_QUERY_LIMIT: u32 = 50;
const MAX_GRAPH_QUERY_LIMIT: u32 = 200;
const GRAPH_QUERY_EVIDENCE_LIMIT: i64 = 16;
const GRAPH_QUERY_FACTS_SQL: &str = "\
SELECT
	fact_id,
	scope,
	agent_id AS actor,
	predicate,
	predicate_id,
	object_entity_id,
	object_entity.canonical AS object_canonical,
	object_entity.kind AS object_kind,
	object_value,
	valid_from,
	valid_to,
	COALESCE(
		(SELECT ARRAY_AGG(e.note_id ORDER BY e.created_at ASC, e.note_id ASC)
		 FROM (
		 	SELECT note_id, created_at
		 	FROM graph_fact_evidence
		 	WHERE fact_id = gf.fact_id
		 	ORDER BY created_at ASC, note_id ASC
		 	LIMIT $9
		 ) e),
		'{}'::uuid[]
	) AS evidence_note_ids
FROM graph_facts AS gf
LEFT JOIN graph_entities AS object_entity
	ON object_entity.entity_id = gf.object_entity_id
	AND object_entity.tenant_id = gf.tenant_id
	AND object_entity.project_id = gf.project_id
WHERE gf.tenant_id = $1
	AND (gf.project_id = $2 OR (gf.project_id = $10 AND gf.scope = 'org_shared'))
	AND gf.subject_entity_id = $3
	AND gf.scope = ANY($4::text[])
	AND gf.valid_from <= $5
	AND (gf.valid_to IS NULL OR gf.valid_to > $5)
	AND ($11::uuid IS NULL OR gf.predicate_id = $11)
	AND (
		(gf.scope = 'agent_private' AND gf.agent_id = $6)
		OR (gf.scope <> 'agent_private' AND (
			gf.agent_id = $6 OR (gf.scope || ':' || gf.agent_id) = ANY($7::text[])
		))
	)
ORDER BY gf.valid_from DESC, gf.fact_id ASC
LIMIT $8";

/// Subject selector used by graph-query APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GraphQueryEntityRef {
	/// Resolve the subject by entity identifier.
	EntityId {
		/// Entity identifier to resolve.
		entity_id: Uuid,
	},
	/// Resolve the subject by canonical or alias surface.
	Surface {
		/// Canonical or alias surface to resolve.
		surface: String,
	},
}

/// Predicate selector used by graph-query APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GraphQueryPredicateRef {
	/// Resolve the predicate by predicate identifier.
	PredicateId {
		/// Predicate identifier to resolve.
		predicate_id: Uuid,
	},
	/// Resolve the predicate by canonical or alias surface.
	Surface {
		/// Canonical or alias surface to resolve.
		surface: String,
	},
}

/// Request payload for graph-query lookups.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GraphQueryRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Subject entity selector.
	pub subject: GraphQueryEntityRef,

	/// Optional predicate selector used to narrow the results.
	pub predicate: Option<GraphQueryPredicateRef>,

	/// Optional requested scopes.
	pub scopes: Option<Vec<String>>,
	#[serde(with = "crate::time_serde::option")]
	/// Point-in-time view for temporal facts.
	pub as_of: Option<OffsetDateTime>,
	/// Optional maximum number of returned facts.
	pub limit: Option<u32>,
	/// When true, includes explain metadata.
	pub explain: Option<bool>,
}

/// Response payload for graph-query lookups.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryResponse {
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time view used for the query.
	pub as_of: OffsetDateTime,
	/// Resolved subject entity.
	pub subject: GraphQueryEntity,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Resolved predicate, when the request filtered by predicate.
	pub predicate: Option<GraphQueryPredicate>,
	/// Effective scopes used for the query.
	pub scopes: Vec<String>,
	/// Whether the result set was truncated by the limit.
	pub truncated: bool,
	/// Returned fact rows.
	pub facts: Vec<GraphQueryFact>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional explain metadata.
	pub explain: Option<GraphQueryExplain>,
}

/// Resolved graph entity reference.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Canonical entity surface.
	pub canonical: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
}

/// Resolved graph predicate reference.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryPredicate {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Canonical predicate surface.
	pub canonical: String,
}

/// One graph fact returned by the query.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryFact {
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Scope key for the fact.
	pub scope: String,
	/// Agent that emitted the fact.
	pub actor: String,
	/// Predicate surface recorded on the fact.
	pub predicate: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Resolved predicate identifier, when available.
	pub predicate_id: Option<Uuid>,
	#[serde(with = "crate::time_serde")]
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// End of the fact validity window, if superseded.
	pub valid_to: Option<OffsetDateTime>,
	/// Object payload for the fact.
	pub object: GraphQueryObject,
	/// Evidence note identifiers supporting the fact.
	pub evidence_note_ids: Vec<Uuid>,
}

/// Object payload returned for a graph fact.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryObject {
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Entity-shaped object value.
	pub entity: Option<GraphQueryObjectEntity>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Scalar object value.
	pub value: Option<String>,
}

/// Resolved entity payload for a graph-fact object.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryObjectEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Canonical entity surface.
	pub canonical: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
}

/// Explain metadata for a graph-query response.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryExplain {
	/// Explain schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time view used for the query.
	pub as_of: OffsetDateTime,
	/// Requested result limit.
	pub requested_limit: u32,
	/// Scopes allowed by the read profile.
	pub allowed_scopes: Vec<String>,
	/// Scopes effectively queried after request filtering.
	pub effective_scopes: Vec<String>,
	/// Number of rows read from storage.
	pub queried_rows: usize,
	/// Number of rows returned to the caller.
	pub returned_rows: usize,
	/// Whether the result set was truncated by the limit.
	pub truncated: bool,
}

#[derive(Debug)]
struct PreparedGraphQuery {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	subject: GraphQueryEntityRef,
	predicate: Option<GraphQueryPredicateRef>,
	requested_scopes: Vec<String>,
	as_of: OffsetDateTime,
	limit: usize,
	explain: bool,
}

#[derive(Debug)]
struct ResolvedGraphQuerySubject {
	entity_id: Uuid,
	canonical: String,
	kind: Option<String>,
}

#[derive(Debug)]
struct ResolvedGraphQueryPredicate {
	id: Uuid,
	canonical: String,
}

#[derive(Debug)]
struct GraphQueryRowsFetchParams<'a> {
	tenant_id: &'a str,
	project_id: &'a str,
	subject_entity_id: Uuid,
	scopes: &'a [String],
	as_of: OffsetDateTime,
	actor: &'a str,
	shared_scope_keys: &'a [String],
	predicate_id: Option<Uuid>,
	limit_plus_one: i64,
}

#[derive(Debug, FromRow)]
struct GraphQueryFactRow {
	fact_id: Uuid,
	scope: String,
	actor: String,
	predicate: String,
	predicate_id: Option<Uuid>,
	object_entity_id: Option<Uuid>,
	object_canonical: Option<String>,
	object_kind: Option<String>,
	object_value: Option<String>,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
	evidence_note_ids: Vec<Uuid>,
}

impl ElfService {
	/// Resolves a subject and returns active graph facts visible to the caller.
	pub async fn graph_query(&self, req: GraphQueryRequest) -> Result<GraphQueryResponse> {
		let prepared = validate_graph_query_request(req)?;
		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, prepared.read_profile.as_str())?;
		let effective_scopes =
			resolve_effective_scopes(&allowed_scopes, prepared.requested_scopes.as_slice())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope.trim() == "org_shared");
		let mut conn = self.db.pool.acquire().await?;
		let subject =
			resolve_subject(&mut conn, &prepared.tenant_id, &prepared.project_id, prepared.subject)
				.await?;
		let predicate = resolve_predicate(
			&mut conn,
			&prepared.tenant_id,
			&prepared.project_id,
			prepared.predicate,
		)
		.await?;
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			conn.as_mut(),
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let shared_scope_keys: Vec<String> = shared_grants
			.into_iter()
			.map(|item| format!("{}:{}", item.scope, item.space_owner_agent_id))
			.collect();
		let predicate_id = predicate.as_ref().map(|predicate| predicate.id);
		let rows = fetch_graph_query_rows(
			&mut conn,
			GraphQueryRowsFetchParams {
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				subject_entity_id: subject.entity_id,
				scopes: effective_scopes.as_slice(),
				as_of: prepared.as_of,
				actor: prepared.agent_id.as_str(),
				shared_scope_keys: shared_scope_keys.as_slice(),
				predicate_id,
				limit_plus_one: (prepared.limit as i64) + 1,
			},
		)
		.await?;
		let facts: Vec<GraphQueryFact> = rows
			.into_iter()
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
					object,
					evidence_note_ids: row.evidence_note_ids,
				}
			})
			.collect();
		let queried_rows = facts.len();
		let (facts, truncated) = truncate_graph_query_facts(facts, prepared.limit);
		let explain = if prepared.explain {
			Some(build_graph_query_explain(
				prepared.as_of,
				&allowed_scopes,
				&effective_scopes,
				prepared.limit,
				queried_rows,
				facts.len(),
				truncated,
			))
		} else {
			None
		};

		Ok(GraphQueryResponse {
			as_of: prepared.as_of,
			subject: GraphQueryEntity {
				entity_id: subject.entity_id,
				canonical: subject.canonical,
				kind: subject.kind,
			},
			predicate: predicate.map(|resolved| GraphQueryPredicate {
				predicate_id: resolved.id,
				canonical: resolved.canonical,
			}),
			scopes: effective_scopes,
			truncated,
			facts,
			explain,
		})
	}
}

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

fn validate_graph_query_request(req: GraphQueryRequest) -> Result<PreparedGraphQuery> {
	let tenant_id = normalize_required_field(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required_field(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required_field(req.agent_id.as_str(), "agent_id")?;
	let read_profile = normalize_required_field(req.read_profile.as_str(), "read_profile")?;
	let subject = match req.subject {
		GraphQueryEntityRef::EntityId { entity_id } => GraphQueryEntityRef::EntityId { entity_id },
		GraphQueryEntityRef::Surface { surface } => {
			let surface = normalize_required_field(surface.as_str(), "subject.surface")?;

			GraphQueryEntityRef::Surface { surface }
		},
	};
	let predicate = match req.predicate {
		Some(GraphQueryPredicateRef::PredicateId { predicate_id }) =>
			Some(GraphQueryPredicateRef::PredicateId { predicate_id }),
		Some(GraphQueryPredicateRef::Surface { surface }) => {
			let surface = normalize_required_field(surface.as_str(), "predicate.surface")?;

			Some(GraphQueryPredicateRef::Surface { surface })
		},
		None => None,
	};
	let requested_scopes = normalize_scopes(req.scopes)?;
	let limit = req.limit.unwrap_or(DEFAULT_GRAPH_QUERY_LIMIT);

	if !matches!(limit, 1..=MAX_GRAPH_QUERY_LIMIT) {
		return Err(Error::InvalidRequest {
			message: format!("limit must be between 1 and {MAX_GRAPH_QUERY_LIMIT}."),
		});
	}

	Ok(PreparedGraphQuery {
		tenant_id,
		project_id,
		agent_id,
		read_profile,
		subject,
		predicate,
		requested_scopes,
		as_of: req.as_of.unwrap_or_else(OffsetDateTime::now_utc),
		limit: limit as usize,
		explain: req.explain.unwrap_or(false),
	})
}

fn normalize_required_field(value: &str, field: &str) -> Result<String> {
	let trimmed = value.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} is required.") });
	}

	Ok(trimmed.to_string())
}

fn normalize_scopes(scopes: Option<Vec<String>>) -> Result<Vec<String>> {
	let scopes = scopes.unwrap_or_default();
	let mut seen = std::collections::HashSet::new();
	let mut normalized = Vec::new();

	for scope in scopes {
		let scope = scope.trim().to_string();

		if scope.is_empty() {
			return Err(Error::InvalidRequest {
				message: "scopes entries must be non-empty strings.".to_string(),
			});
		}
		if seen.insert(scope.clone()) {
			normalized.push(scope);
		}
	}

	Ok(normalized)
}

async fn resolve_subject(
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

async fn resolve_predicate(
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

async fn fetch_graph_query_rows(
	conn: &mut PgConnection,
	params: GraphQueryRowsFetchParams<'_>,
) -> Result<Vec<GraphQueryFactRow>> {
	let GraphQueryRowsFetchParams {
		tenant_id,
		project_id,
		subject_entity_id,
		scopes,
		as_of,
		actor,
		shared_scope_keys,
		predicate_id,
		limit_plus_one,
	} = params;
	let rows = sqlx::query_as::<_, GraphQueryFactRow>(GRAPH_QUERY_FACTS_SQL)
		.bind(tenant_id)
		.bind(project_id)
		.bind(subject_entity_id)
		.bind(scopes)
		.bind(as_of)
		.bind(actor)
		.bind(shared_scope_keys)
		.bind(limit_plus_one)
		.bind(GRAPH_QUERY_EVIDENCE_LIMIT)
		.bind(crate::access::ORG_PROJECT_ID)
		.bind(predicate_id)
		.fetch_all(conn)
		.await?;

	Ok(rows)
}

#[cfg(test)]
mod tests {
	use std::collections::HashSet;

	use uuid::Uuid;

	use crate::{
		ELF_GRAPH_QUERY_SCHEMA_V1, Error, GraphQueryFact, GraphQueryObject, GraphQueryObjectEntity,
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
		let allowed = vec![
			"agent_private".to_string(),
			"project_shared".to_string(),
			"org_shared".to_string(),
		];
		let requested = vec!["project_shared".to_string(), "project_shared".to_string()];
		let resolved =
			graph_query::resolve_effective_scopes(&allowed, &requested).expect("valid scopes");
		let deduped: HashSet<_> = resolved.iter().collect();

		assert_eq!(resolved, vec!["project_shared".to_string()]);
		assert_eq!(deduped.len(), 1);
	}
}
