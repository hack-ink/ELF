//! Source-backed graph topic-map reports.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	access::{self, ORG_PROJECT_ID},
	graph::RelationTemporalStatus,
	graph_query::{
		self, GraphQueryEntityRef, GraphQueryObject, GraphQueryObjectEntity, GraphQueryPredicateRef,
	},
	search,
};
use elf_storage::{graph, models::GraphEntity};

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

/// Request payload for a graph topic-map report.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GraphReportRequest {
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
	/// Optional predicate selector used to narrow the report.
	pub predicate: Option<GraphQueryPredicateRef>,
	/// Optional requested scopes.
	pub scopes: Option<Vec<String>>,
	#[serde(with = "crate::time_serde::option")]
	/// Point-in-time used for current, historical, and future classification.
	pub as_of: Option<OffsetDateTime>,
	/// Optional maximum number of returned facts.
	pub limit: Option<u32>,
	/// When true, includes explain metadata.
	pub explain: Option<bool>,
}

/// Response payload for a graph topic-map report.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportResponse {
	/// Report schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time view used for temporal classification.
	pub as_of: OffsetDateTime,
	/// Resolved subject entity.
	pub subject: GraphReportEntity,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Resolved predicate, when the request filtered by predicate.
	pub predicate: Option<GraphReportPredicate>,
	/// Effective scopes used for the report.
	pub scopes: Vec<String>,
	/// Aggregate report counters.
	pub summary: GraphReportSummary,
	/// Topic map projection of the graph facts.
	pub topic_map: GraphTopicMap,
	/// Returned fact rows.
	pub facts: Vec<GraphReportFact>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional explain metadata.
	pub explain: Option<GraphReportExplain>,
}

/// Resolved graph entity reference.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportEntity {
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
pub struct GraphReportPredicate {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Canonical predicate surface.
	pub canonical: String,
}

/// Aggregate counters for graph reports.
#[derive(Clone, Debug, Default, Serialize)]
pub struct GraphReportSummary {
	/// Number of returned facts.
	pub fact_count: usize,
	/// Number of facts current at `as_of`.
	pub current_count: usize,
	/// Number of facts historical at `as_of`.
	pub historical_count: usize,
	/// Number of facts whose validity starts after `as_of`.
	pub future_count: usize,
	/// Number of facts with at least one evidence note link.
	pub sourced_count: usize,
	/// Number of facts still backed by pending or unresolved predicate vocabulary.
	pub inferred_count: usize,
	/// Number of facts that conflict under a single-cardinality predicate.
	pub ambiguous_count: usize,
	/// Number of stale facts, currently equivalent to historical facts.
	pub stale_count: usize,
	/// Number of facts linked to a superseding replacement.
	pub superseded_count: usize,
	/// Total evidence note links returned with the facts.
	pub evidence_link_count: usize,
}

/// One graph fact returned by a graph report.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportFact {
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
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Predicate registry status, when available.
	pub predicate_status: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Predicate registry cardinality, when available.
	pub predicate_cardinality: Option<String>,
	#[serde(with = "crate::time_serde")]
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// End of the fact validity window, if superseded or explicitly bounded.
	pub valid_to: Option<OffsetDateTime>,
	/// Temporal state for the fact relative to report `as_of`.
	pub temporal_status: RelationTemporalStatus,
	/// Object payload for the fact.
	pub object: GraphQueryObject,
	/// Evidence note identifiers supporting the fact.
	pub evidence_note_ids: Vec<Uuid>,
	/// Replacement fact ids that supersede this fact.
	pub superseded_by_fact_ids: Vec<Uuid>,
	/// Older fact ids superseded by this fact.
	pub supersedes_fact_ids: Vec<Uuid>,
	/// Source-backed report status markers.
	pub status_markers: Vec<String>,
}

/// Topic-map projection for graph reports.
#[derive(Clone, Debug, Serialize)]
pub struct GraphTopicMap {
	/// Topic-map nodes.
	pub nodes: Vec<GraphTopicNode>,
	/// Topic-map edges, one per returned fact.
	pub edges: Vec<GraphTopicEdge>,
}

/// Topic-map node.
#[derive(Clone, Debug, Serialize)]
pub struct GraphTopicNode {
	/// Stable node identifier.
	pub node_id: String,
	/// Human-readable node label.
	pub label: String,
	/// Node type such as subject, entity, or value.
	pub node_type: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
}

/// Topic-map edge.
#[derive(Clone, Debug, Serialize)]
pub struct GraphTopicEdge {
	/// Backing fact identifier.
	pub fact_id: Uuid,
	/// Source topic node identifier.
	pub source_node_id: String,
	/// Target topic node identifier.
	pub target_node_id: String,
	/// Predicate label.
	pub predicate: String,
	/// Temporal state for the edge.
	pub temporal_status: RelationTemporalStatus,
	/// Source-backed report status markers.
	pub status_markers: Vec<String>,
	/// Evidence note identifiers supporting the edge.
	pub evidence_note_ids: Vec<Uuid>,
}

/// Explain metadata for graph reports.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportExplain {
	/// Explain schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time used for classification.
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
struct PreparedGraphReport {
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
struct ResolvedGraphReportSubject {
	entity_id: Uuid,
	canonical: String,
	kind: Option<String>,
}

#[derive(Debug)]
struct ResolvedGraphReportPredicate {
	id: Uuid,
	canonical: String,
}

#[derive(Debug)]
struct GraphReportRowsFetchParams<'a> {
	tenant_id: &'a str,
	project_id: &'a str,
	subject_entity_id: Uuid,
	scopes: &'a [String],
	actor: &'a str,
	shared_scope_keys: &'a [String],
	predicate_id: Option<Uuid>,
	limit_plus_one: i64,
}

#[derive(Debug, FromRow)]
struct GraphReportFactRow {
	fact_id: Uuid,
	scope: String,
	actor: String,
	predicate: String,
	predicate_id: Option<Uuid>,
	predicate_status: Option<String>,
	predicate_cardinality: Option<String>,
	object_entity_id: Option<Uuid>,
	object_canonical: Option<String>,
	object_kind: Option<String>,
	object_value: Option<String>,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
	evidence_note_ids: Vec<Uuid>,
	superseded_by_fact_ids: Vec<Uuid>,
	supersedes_fact_ids: Vec<Uuid>,
}

impl ElfService {
	/// Builds a source-backed graph report for one subject entity.
	pub async fn graph_report(&self, req: GraphReportRequest) -> Result<GraphReportResponse> {
		let prepared = validate_graph_report_request(req)?;
		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, prepared.read_profile.as_str())?;
		let effective_scopes = graph_query::resolve_effective_scopes(
			&allowed_scopes,
			prepared.requested_scopes.as_slice(),
		)?;
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
		let rows = fetch_graph_report_rows(
			&mut conn,
			GraphReportRowsFetchParams {
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				subject_entity_id: subject.entity_id,
				scopes: effective_scopes.as_slice(),
				actor: prepared.agent_id.as_str(),
				shared_scope_keys: shared_scope_keys.as_slice(),
				predicate_id,
				limit_plus_one: (prepared.limit as i64) + 1,
			},
		)
		.await?;
		let queried_rows = rows.len();
		let (rows, truncated) = truncate_report_rows(rows, prepared.limit);
		let facts = build_report_facts(rows, prepared.as_of);
		let summary = summarize_report_facts(&facts);
		let topic_map = build_topic_map(&subject, &facts);
		let explain = if prepared.explain {
			Some(GraphReportExplain {
				schema: ELF_GRAPH_REPORT_SCHEMA_V1.to_string(),
				as_of: prepared.as_of,
				requested_limit: prepared.limit as u32,
				allowed_scopes,
				effective_scopes: effective_scopes.clone(),
				queried_rows,
				returned_rows: facts.len(),
				truncated,
			})
		} else {
			None
		};

		Ok(GraphReportResponse {
			schema: ELF_GRAPH_REPORT_SCHEMA_V1.to_string(),
			as_of: prepared.as_of,
			subject: GraphReportEntity {
				entity_id: subject.entity_id,
				canonical: subject.canonical,
				kind: subject.kind,
			},
			predicate: predicate.map(|resolved| GraphReportPredicate {
				predicate_id: resolved.id,
				canonical: resolved.canonical,
			}),
			scopes: effective_scopes,
			summary,
			topic_map,
			facts,
			explain,
		})
	}
}

fn validate_graph_report_request(req: GraphReportRequest) -> Result<PreparedGraphReport> {
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
	let limit = req.limit.unwrap_or(DEFAULT_GRAPH_REPORT_LIMIT);

	if !matches!(limit, 1..=MAX_GRAPH_REPORT_LIMIT) {
		return Err(Error::InvalidRequest {
			message: format!("limit must be between 1 and {MAX_GRAPH_REPORT_LIMIT}."),
		});
	}

	Ok(PreparedGraphReport {
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
	let mut seen = BTreeSet::new();
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

fn truncate_report_rows(
	mut rows: Vec<GraphReportFactRow>,
	limit: usize,
) -> (Vec<GraphReportFactRow>, bool) {
	let truncated = rows.len() > limit;

	if truncated {
		rows.truncate(limit);
	}

	(rows, truncated)
}

fn build_report_facts(
	rows: Vec<GraphReportFactRow>,
	as_of: OffsetDateTime,
) -> Vec<GraphReportFact> {
	let rows: Vec<GraphReportFactRow> =
		rows.into_iter().filter(|row| !row.evidence_note_ids.is_empty()).collect();
	let current_single_counts = current_single_predicate_counts(&rows, as_of);

	rows.into_iter()
		.map(|row| {
			let temporal_status =
				crate::graph::relation_temporal_status(row.valid_from, row.valid_to, as_of);
			let object = graph_object(&row);
			let predicate_key = predicate_group_key(&row);
			let ambiguous = temporal_status == RelationTemporalStatus::Current
				&& row.predicate_cardinality.as_deref() == Some("single")
				&& current_single_counts.get(&predicate_key).copied().unwrap_or(0) > 1;
			let status_markers = report_status_markers(&row, temporal_status, ambiguous);

			GraphReportFact {
				fact_id: row.fact_id,
				scope: row.scope,
				actor: row.actor,
				predicate: row.predicate,
				predicate_id: row.predicate_id,
				predicate_status: row.predicate_status,
				predicate_cardinality: row.predicate_cardinality,
				valid_from: row.valid_from,
				valid_to: row.valid_to,
				temporal_status,
				object,
				evidence_note_ids: row.evidence_note_ids,
				superseded_by_fact_ids: row.superseded_by_fact_ids,
				supersedes_fact_ids: row.supersedes_fact_ids,
				status_markers,
			}
		})
		.collect()
}

fn current_single_predicate_counts(
	rows: &[GraphReportFactRow],
	as_of: OffsetDateTime,
) -> BTreeMap<String, usize> {
	let mut counts = BTreeMap::new();

	for row in rows {
		if row.predicate_cardinality.as_deref() != Some("single") {
			continue;
		}
		if crate::graph::relation_temporal_status(row.valid_from, row.valid_to, as_of)
			!= RelationTemporalStatus::Current
		{
			continue;
		}

		*counts.entry(predicate_group_key(row)).or_insert(0) += 1;
	}

	counts
}

fn predicate_group_key(row: &GraphReportFactRow) -> String {
	row.predicate_id
		.map(|id| id.to_string())
		.unwrap_or_else(|| format!("surface:{}", row.predicate))
}

fn graph_object(row: &GraphReportFactRow) -> GraphQueryObject {
	if let Some(entity_id) = row.object_entity_id {
		return GraphQueryObject {
			entity: Some(GraphQueryObjectEntity {
				entity_id,
				canonical: row.object_canonical.clone().unwrap_or_default(),
				kind: row.object_kind.clone(),
			}),
			value: None,
		};
	}

	GraphQueryObject { entity: None, value: row.object_value.clone() }
}

fn report_status_markers(
	row: &GraphReportFactRow,
	temporal_status: RelationTemporalStatus,
	ambiguous: bool,
) -> Vec<String> {
	let mut markers = Vec::new();

	if row.evidence_note_ids.is_empty() {
		markers.push("unsupported".to_string());
	} else {
		markers.push("sourced".to_string());
	}
	if row.predicate_status.as_deref() != Some("active") {
		markers.push("inferred".to_string());
	}
	if temporal_status == RelationTemporalStatus::Historical {
		markers.push("stale".to_string());
	}
	if !row.superseded_by_fact_ids.is_empty() {
		markers.push("superseded".to_string());
	}
	if ambiguous {
		markers.push("ambiguous".to_string());
	}

	markers
}

fn summarize_report_facts(facts: &[GraphReportFact]) -> GraphReportSummary {
	let mut summary = GraphReportSummary { fact_count: facts.len(), ..Default::default() };

	for fact in facts {
		match fact.temporal_status {
			RelationTemporalStatus::Current => summary.current_count += 1,
			RelationTemporalStatus::Historical => summary.historical_count += 1,
			RelationTemporalStatus::Future => summary.future_count += 1,
		}

		if !fact.evidence_note_ids.is_empty() {
			summary.sourced_count += 1;
		}
		if fact.status_markers.iter().any(|marker| marker == "inferred") {
			summary.inferred_count += 1;
		}
		if fact.status_markers.iter().any(|marker| marker == "ambiguous") {
			summary.ambiguous_count += 1;
		}
		if fact.status_markers.iter().any(|marker| marker == "stale") {
			summary.stale_count += 1;
		}
		if fact.status_markers.iter().any(|marker| marker == "superseded") {
			summary.superseded_count += 1;
		}

		summary.evidence_link_count += fact.evidence_note_ids.len();
	}

	summary
}

fn build_topic_map(
	subject: &ResolvedGraphReportSubject,
	facts: &[GraphReportFact],
) -> GraphTopicMap {
	let subject_node_id = format!("entity:{}", subject.entity_id);
	let mut nodes = BTreeMap::new();

	nodes.insert(
		subject_node_id.clone(),
		GraphTopicNode {
			node_id: subject_node_id.clone(),
			label: subject.canonical.clone(),
			node_type: "subject".to_string(),
			kind: subject.kind.clone(),
		},
	);

	let edges = facts
		.iter()
		.map(|fact| {
			let (target_node_id, label, kind, node_type) = match &fact.object.entity {
				Some(entity) => (
					format!("entity:{}", entity.entity_id),
					entity.canonical.clone(),
					entity.kind.clone(),
					"entity".to_string(),
				),
				None => (
					format!("value:{}", fact.object.value.as_deref().unwrap_or_default()),
					fact.object.value.clone().unwrap_or_default(),
					None,
					"value".to_string(),
				),
			};

			nodes.entry(target_node_id.clone()).or_insert_with(|| GraphTopicNode {
				node_id: target_node_id.clone(),
				label,
				node_type,
				kind,
			});
			GraphTopicEdge {
				fact_id: fact.fact_id,
				source_node_id: subject_node_id.clone(),
				target_node_id,
				predicate: fact.predicate.clone(),
				temporal_status: fact.temporal_status,
				status_markers: fact.status_markers.clone(),
				evidence_note_ids: fact.evidence_note_ids.clone(),
			}
		})
		.collect();

	GraphTopicMap { nodes: nodes.into_values().collect(), edges }
}

async fn resolve_subject(
	conn: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	subject: GraphQueryEntityRef,
) -> Result<ResolvedGraphReportSubject> {
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

			Ok(ResolvedGraphReportSubject {
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

			Ok(ResolvedGraphReportSubject {
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
) -> Result<Option<ResolvedGraphReportPredicate>> {
	match predicate {
		Some(GraphQueryPredicateRef::PredicateId { predicate_id }) => {
			let Some(row) = graph::get_predicate_by_id(conn, predicate_id).await? else {
				return Err(Error::NotFound {
					message: format!("graph predicate not found; predicate_id={predicate_id}"),
				});
			};

			Ok(Some(ResolvedGraphReportPredicate {
				id: row.predicate_id,
				canonical: row.canonical,
			}))
		},
		Some(GraphQueryPredicateRef::Surface { surface }) => {
			let Some(row) =
				graph::resolve_predicate_no_register(conn, tenant_id, project_id, &surface).await?
			else {
				return Err(Error::NotFound {
					message: format!("graph predicate not found for surface={surface}"),
				});
			};

			Ok(Some(ResolvedGraphReportPredicate {
				id: row.predicate_id,
				canonical: row.canonical,
			}))
		},
		None => Ok(None),
	}
}

async fn fetch_graph_report_rows(
	conn: &mut PgConnection,
	params: GraphReportRowsFetchParams<'_>,
) -> Result<Vec<GraphReportFactRow>> {
	let rows = sqlx::query_as::<_, GraphReportFactRow>(GRAPH_REPORT_FACTS_SQL)
		.bind(params.tenant_id)
		.bind(params.project_id)
		.bind(params.subject_entity_id)
		.bind(params.scopes)
		.bind(OffsetDateTime::now_utc())
		.bind(params.actor)
		.bind(params.shared_scope_keys)
		.bind(params.limit_plus_one)
		.bind(GRAPH_REPORT_EVIDENCE_LIMIT)
		.bind(ORG_PROJECT_ID)
		.bind(params.predicate_id)
		.fetch_all(conn)
		.await?;

	Ok(rows)
}

#[cfg(test)]
mod tests {
	use time::OffsetDateTime;
	use uuid::Uuid;

	use crate::{
		RelationTemporalStatus,
		graph_report::{self, GraphReportFactRow},
	};

	fn ts(value: i64) -> OffsetDateTime {
		OffsetDateTime::from_unix_timestamp(value).expect("valid timestamp")
	}

	fn row(
		raw_id: u128,
		object_value: &str,
		valid_from: i64,
		valid_to: Option<i64>,
		predicate_status: &str,
		cardinality: &str,
		superseded_by: Vec<Uuid>,
	) -> GraphReportFactRow {
		GraphReportFactRow {
			fact_id: Uuid::from_u128(raw_id),
			scope: "agent_private".to_string(),
			actor: "agent".to_string(),
			predicate: "works at".to_string(),
			predicate_id: Some(Uuid::from_u128(999)),
			predicate_status: Some(predicate_status.to_string()),
			predicate_cardinality: Some(cardinality.to_string()),
			object_entity_id: None,
			object_canonical: None,
			object_kind: None,
			object_value: Some(object_value.to_string()),
			valid_from: ts(valid_from),
			valid_to: valid_to.map(ts),
			evidence_note_ids: vec![Uuid::from_u128(raw_id + 10_000)],
			superseded_by_fact_ids: superseded_by,
			supersedes_fact_ids: vec![],
		}
	}

	#[test]
	fn graph_report_classifies_temporal_source_and_supersession_markers() {
		let replacement_id = Uuid::from_u128(2);
		let facts = graph_report::build_report_facts(
			vec![
				row(1, "Initech", 10, Some(20), "active", "single", vec![replacement_id]),
				row(2, "Globex", 20, None, "active", "single", vec![]),
				row(3, "Umbrella", 30, None, "pending", "single", vec![]),
			],
			ts(25),
		);
		let summary = graph_report::summarize_report_facts(&facts);

		assert_eq!(summary.fact_count, 3);
		assert_eq!(summary.current_count, 1);
		assert_eq!(summary.historical_count, 1);
		assert_eq!(summary.future_count, 1);
		assert_eq!(summary.sourced_count, 3);
		assert_eq!(summary.inferred_count, 1);
		assert_eq!(summary.stale_count, 1);
		assert_eq!(summary.superseded_count, 1);
		assert_eq!(summary.evidence_link_count, 3);
		assert_eq!(facts[0].temporal_status, RelationTemporalStatus::Historical);
		assert!(facts[0].status_markers.iter().any(|marker| marker == "superseded"));
		assert!(facts[2].status_markers.iter().any(|marker| marker == "inferred"));
	}

	#[test]
	fn graph_report_suppresses_facts_without_readable_evidence() {
		let mut deleted_source =
			row(1, "Deleted Source Inc.", 10, None, "active", "single", vec![]);

		deleted_source.evidence_note_ids = vec![];

		let facts = graph_report::build_report_facts(
			vec![
				deleted_source,
				row(2, "Active Source Inc.", 20, None, "active", "single", vec![]),
			],
			ts(25),
		);

		assert_eq!(facts.len(), 1);
		assert_eq!(facts[0].fact_id, Uuid::from_u128(2));
		assert_eq!(facts[0].object.value.as_deref(), Some("Active Source Inc."));
	}

	#[test]
	fn graph_topic_map_preserves_fact_edges_and_source_markers() {
		let subject = super::ResolvedGraphReportSubject {
			entity_id: Uuid::from_u128(42),
			canonical: "Alice".to_string(),
			kind: Some("person".to_string()),
		};
		let facts = graph_report::build_report_facts(
			vec![row(1, "Globex", 20, None, "active", "single", vec![])],
			ts(25),
		);
		let topic_map = graph_report::build_topic_map(&subject, &facts);

		assert_eq!(topic_map.nodes.len(), 2);
		assert_eq!(topic_map.edges.len(), 1);
		assert_eq!(topic_map.edges[0].predicate, "works at");
		assert_eq!(topic_map.edges[0].temporal_status, RelationTemporalStatus::Current);
		assert!(topic_map.edges[0].status_markers.iter().any(|marker| marker == "sourced"));
	}
}
