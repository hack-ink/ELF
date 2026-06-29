use sqlx::Row;

use crate::search::{
	self, HashMap, PgPool, Result, SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1, SearchExplainTrajectory,
	SearchExplainTrajectoryMatch, SearchExplainTrajectoryStage, SearchTrajectoryStage,
	SearchTrajectoryStageItem, SearchTrajectorySummary, Uuid, Value,
};

pub(super) async fn load_trace_trajectory_summary(
	pool: &PgPool,
	trace_id: Uuid,
) -> Result<Option<SearchTrajectorySummary>> {
	let stages = load_trace_trajectory_stages(pool, trace_id).await?;

	if stages.is_empty() {
		Ok(None)
	} else {
		Ok(Some(search::build_trajectory_summary_from_stages(stages.as_slice())))
	}
}

pub(super) async fn load_trace_trajectory_stages(
	pool: &PgPool,
	trace_id: Uuid,
) -> Result<Vec<SearchTrajectoryStage>> {
	let rows = sqlx::query(
		"\
	SELECT
	s.stage_id,
	s.stage_order,
	s.stage_name,
	s.stage_payload,
	i.item_id,
	i.note_id,
	i.chunk_id,
	i.metrics
FROM search_trace_stages s
LEFT JOIN search_trace_stage_items i ON i.stage_id = s.stage_id
WHERE s.trace_id = $1
ORDER BY s.stage_order ASC, i.item_id ASC NULLS LAST, i.note_id ASC NULLS LAST",
	)
	.bind(trace_id)
	.fetch_all(pool)
	.await?;
	let mut stages = Vec::new();
	let mut stage_pos_by_id: HashMap<Uuid, usize> = HashMap::new();

	for row in rows {
		let stage_id: Uuid = row.try_get("stage_id")?;
		let idx = if let Some(idx) = stage_pos_by_id.get(&stage_id).copied() {
			idx
		} else {
			let stage_order: i32 = row.try_get("stage_order")?;
			let stage_name: String = row.try_get("stage_name")?;
			let stage_payload: Value = row.try_get("stage_payload")?;
			let idx = stages.len();

			stages.push(SearchTrajectoryStage {
				stage_order: stage_order as u32,
				stage_name,
				stage_payload,
				items: Vec::new(),
			});
			stage_pos_by_id.insert(stage_id, idx);

			idx
		};
		let item_metrics: Option<Value> = row.try_get("metrics")?;

		if let Some(metrics) = item_metrics {
			stages[idx].items.push(SearchTrajectoryStageItem {
				item_id: row.try_get("item_id")?,
				note_id: row.try_get("note_id")?,
				chunk_id: row.try_get("chunk_id")?,
				metrics,
			});
		}
	}

	Ok(stages)
}

pub(super) async fn load_item_trajectory(
	pool: &PgPool,
	trace_id: Uuid,
	item_id: Uuid,
	note_id: Uuid,
	trace_item_chunk_id: Option<Uuid>,
) -> Result<Option<SearchExplainTrajectory>> {
	let rows = sqlx::query(
		"\
SELECT
	s.stage_order,
	s.stage_name,
	s.stage_payload,
	i.item_id,
	i.note_id,
	i.chunk_id,
	i.metrics
FROM search_trace_stages s
LEFT JOIN search_trace_stage_items i
	ON i.stage_id = s.stage_id
	AND (
		i.item_id = $2
		OR (
			i.item_id IS NULL
			AND i.note_id = $3
			AND ($4 IS NULL OR i.chunk_id = $4)
		)
	)
WHERE s.trace_id = $1
ORDER BY s.stage_order ASC, i.item_id ASC NULLS LAST, i.note_id ASC NULLS LAST",
	)
	.bind(trace_id)
	.bind(item_id)
	.bind(note_id)
	.bind(trace_item_chunk_id)
	.fetch_all(pool)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}

	let mut stages = Vec::with_capacity(rows.len());
	let mut stage_pos_by_order: HashMap<u32, usize> = HashMap::new();

	for row in rows {
		let stage_order: i32 = row.try_get("stage_order")?;
		let stage_name: String = row.try_get("stage_name")?;
		let stage_payload: Value = row.try_get("stage_payload")?;
		let stage_order = stage_order as u32;
		let idx = if let Some(idx) = stage_pos_by_order.get(&stage_order).copied() {
			idx
		} else {
			let idx = stages.len();

			stages.push(SearchExplainTrajectoryStage {
				stage_order,
				stage_name,
				stage_payload,
				metrics: serde_json::json!({}),
				match_info: None,
			});
			stage_pos_by_order.insert(stage_order, idx);

			idx
		};
		let item_metrics: Option<Value> = row.try_get("metrics")?;
		let matched_item_id: Option<Uuid> = row.try_get("item_id")?;
		let matched_note_id: Option<Uuid> = row.try_get("note_id")?;
		let matched_chunk_id: Option<Uuid> = row.try_get("chunk_id")?;

		if let Some(metrics) = item_metrics {
			let match_kind = if matched_item_id.is_some() {
				"item_id"
			} else if trace_item_chunk_id.is_some() {
				"note_chunk"
			} else {
				"note"
			};

			stages[idx].match_info = Some(SearchExplainTrajectoryMatch {
				kind: match_kind.to_string(),
				item_id: matched_item_id,
				note_id: matched_note_id,
				chunk_id: matched_chunk_id,
			});
			stages[idx].metrics = metrics;
		}
	}

	Ok(Some(SearchExplainTrajectory {
		schema: SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1.to_string(),
		stages,
	}))
}
