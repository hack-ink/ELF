//! Structured graph query APIs.

mod build;
mod resolution;
mod service;
mod state;
mod storage;
mod types;
mod validation;

pub use types::{
	GraphQueryEntity, GraphQueryEntityRef, GraphQueryExplain, GraphQueryFact, GraphQueryObject,
	GraphQueryObjectEntity, GraphQueryPredicate, GraphQueryPredicateRef, GraphQueryRequest,
	GraphQueryResponse,
};

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result, access::ORG_PROJECT_ID, graph::RelationTemporalStatus};
use build::graph_query_facts_from_rows;
use elf_storage::{graph, models::GraphEntity};
use resolution::{resolve_predicate, resolve_subject};
use state::{
	GraphQueryFactRow, GraphQueryRowsFetchParams, PreparedGraphQuery, ResolvedGraphQueryPredicate,
	ResolvedGraphQuerySubject,
};
use storage::fetch_graph_query_rows;
use validation::validate_graph_query_request;

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
		 	SELECT evidence.note_id, evidence.created_at
		 	FROM graph_fact_evidence evidence
			JOIN memory_notes note ON note.note_id = evidence.note_id
		 	WHERE evidence.fact_id = gf.fact_id
				AND note.tenant_id = gf.tenant_id
				AND note.project_id = gf.project_id
				AND note.status = 'active'
				AND (note.expires_at IS NULL OR note.expires_at > now())
				AND note.scope = ANY($4::text[])
				AND (
					(note.scope = 'agent_private' AND note.agent_id = $6)
					OR (note.scope <> 'agent_private' AND (
						note.agent_id = $6 OR (note.scope || ':' || note.agent_id) = ANY($7::text[])
					))
				)
		 	ORDER BY evidence.created_at ASC, evidence.note_id ASC
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
	AND EXISTS (
		SELECT 1
		FROM graph_fact_evidence evidence
		JOIN memory_notes note ON note.note_id = evidence.note_id
		WHERE evidence.fact_id = gf.fact_id
			AND note.tenant_id = gf.tenant_id
			AND note.project_id = gf.project_id
			AND note.status = 'active'
			AND (note.expires_at IS NULL OR note.expires_at > now())
			AND note.scope = ANY($4::text[])
			AND (
				(note.scope = 'agent_private' AND note.agent_id = $6)
				OR (note.scope <> 'agent_private' AND (
					note.agent_id = $6 OR (note.scope || ':' || note.agent_id) = ANY($7::text[])
				))
			)
	)
	ORDER BY gf.valid_from DESC, gf.fact_id ASC
	LIMIT $8";

pub(crate) fn resolve_effective_scopes(
	allowed_scopes: &[String],
	requested_scopes: &[String],
) -> Result<Vec<String>> {
	build::resolve_effective_scopes(allowed_scopes, requested_scopes)
}

pub(crate) fn truncate_graph_query_facts(
	facts: Vec<GraphQueryFact>,
	limit: usize,
) -> (Vec<GraphQueryFact>, bool) {
	build::truncate_graph_query_facts(facts, limit)
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
	build::build_graph_query_explain(
		as_of,
		allowed_scopes,
		effective_scopes,
		requested_limit,
		queried_rows,
		returned_rows,
		truncated,
	)
}

#[cfg(test)] mod tests;
