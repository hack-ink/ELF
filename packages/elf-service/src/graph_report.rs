//! Source-backed graph topic-map reports.

mod build;
mod resolution;
mod service;
mod state;
mod storage;
mod types;
mod validation;

pub use types::{
	GraphReportEntity, GraphReportExplain, GraphReportFact, GraphReportPredicate,
	GraphReportRequest, GraphReportResponse, GraphReportSummary, GraphTopicEdge, GraphTopicMap,
	GraphTopicNode,
};

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	access::ORG_PROJECT_ID,
	graph::RelationTemporalStatus,
	graph_query::{
		GraphQueryEntityRef, GraphQueryObject, GraphQueryObjectEntity, GraphQueryPredicateRef,
	},
};
use build::{build_report_facts, build_topic_map, summarize_report_facts, truncate_report_rows};
use elf_storage::{graph, models::GraphEntity};
use resolution::{resolve_predicate, resolve_subject};
use state::{
	GraphReportFactRow, GraphReportRowsFetchParams, PreparedGraphReport,
	ResolvedGraphReportPredicate, ResolvedGraphReportSubject,
};
use storage::fetch_graph_report_rows;
use validation::validate_graph_report_request;

/// Schema identifier for graph report responses.
pub const ELF_GRAPH_REPORT_SCHEMA_V1: &str = "elf.graph_report/v1";

const DEFAULT_GRAPH_REPORT_LIMIT: u32 = 100;
const MAX_GRAPH_REPORT_LIMIT: u32 = 500;
const GRAPH_REPORT_EVIDENCE_LIMIT: i64 = 24;
const GRAPH_REPORT_FACTS_SQL: &str = "\
SELECT
	gf.fact_id,
	gf.scope,
	gf.agent_id AS actor,
	gf.predicate,
	gf.predicate_id,
	gp.status AS predicate_status,
	gp.cardinality AS predicate_cardinality,
	gf.object_entity_id,
	object_entity.canonical AS object_canonical,
	object_entity.kind AS object_kind,
	gf.object_value,
	gf.valid_from,
	gf.valid_to,
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
	) AS evidence_note_ids,
	COALESCE(
		(SELECT ARRAY_AGG(s.to_fact_id ORDER BY s.effective_at ASC, s.to_fact_id ASC)
		 FROM graph_fact_supersessions s
		 WHERE s.from_fact_id = gf.fact_id),
		'{}'::uuid[]
	) AS superseded_by_fact_ids,
	COALESCE(
		(SELECT ARRAY_AGG(s.from_fact_id ORDER BY s.effective_at ASC, s.from_fact_id ASC)
		 FROM graph_fact_supersessions s
		 WHERE s.to_fact_id = gf.fact_id),
		'{}'::uuid[]
	) AS supersedes_fact_ids
FROM graph_facts AS gf
LEFT JOIN graph_predicates AS gp
	ON gp.predicate_id = gf.predicate_id
LEFT JOIN graph_entities AS object_entity
	ON object_entity.entity_id = gf.object_entity_id
	AND object_entity.tenant_id = gf.tenant_id
	AND object_entity.project_id = gf.project_id
WHERE gf.tenant_id = $1
	AND (gf.project_id = $2 OR (gf.project_id = $10 AND gf.scope = 'org_shared'))
	AND gf.subject_entity_id = $3
	AND gf.scope = ANY($4::text[])
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

#[cfg(test)] mod tests;
