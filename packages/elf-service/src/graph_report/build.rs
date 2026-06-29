use crate::{
	graph,
	graph_report::{
		BTreeMap, GraphQueryObject, GraphQueryObjectEntity, GraphReportFact, GraphReportFactRow,
		GraphReportSummary, GraphTopicEdge, GraphTopicMap, GraphTopicNode, OffsetDateTime,
		RelationTemporalStatus, ResolvedGraphReportSubject,
	},
};

pub(super) fn truncate_report_rows(
	mut rows: Vec<GraphReportFactRow>,
	limit: usize,
) -> (Vec<GraphReportFactRow>, bool) {
	let truncated = rows.len() > limit;

	if truncated {
		rows.truncate(limit);
	}

	(rows, truncated)
}

pub(super) fn build_report_facts(
	rows: Vec<GraphReportFactRow>,
	as_of: OffsetDateTime,
) -> Vec<GraphReportFact> {
	let rows: Vec<GraphReportFactRow> =
		rows.into_iter().filter(|row| !row.evidence_note_ids.is_empty()).collect();
	let current_single_counts = current_single_predicate_counts(&rows, as_of);

	rows.into_iter()
		.map(|row| {
			let temporal_status =
				graph::relation_temporal_status(row.valid_from, row.valid_to, as_of);
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

pub(super) fn summarize_report_facts(facts: &[GraphReportFact]) -> GraphReportSummary {
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

pub(super) fn build_topic_map(
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

fn current_single_predicate_counts(
	rows: &[GraphReportFactRow],
	as_of: OffsetDateTime,
) -> BTreeMap<String, usize> {
	let mut counts = BTreeMap::new();

	for row in rows {
		if row.predicate_cardinality.as_deref() != Some("single") {
			continue;
		}
		if graph::relation_temporal_status(row.valid_from, row.valid_to, as_of)
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
