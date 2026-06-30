use crate::search::{PgConnection, QueryBuilder, Result, TraceTrajectoryStageRecord, Uuid};

pub(in crate::search::trace_persistence) async fn persist_trace_inline_stages(
	executor: &mut PgConnection,
	trace_id: Uuid,
	stages: Vec<TraceTrajectoryStageRecord>,
) -> Result<()> {
	if stages.is_empty() {
		return Ok(());
	}

	let mut item_records = Vec::new();
	let mut stage_builder = QueryBuilder::new(
		"\
INSERT INTO search_trace_stages (
	stage_id,
	trace_id,
	stage_order,
	stage_name,
	stage_payload,
	created_at
) ",
	);

	stage_builder.push_values(stages, |mut b, stage| {
		for item in stage.items {
			item_records.push((stage.stage_id, item));
		}

		b.push_bind(stage.stage_id)
			.push_bind(trace_id)
			.push_bind(stage.stage_order as i32)
			.push_bind(stage.stage_name)
			.push_bind(stage.stage_payload)
			.push_bind(stage.created_at);
	});
	stage_builder.push(" ON CONFLICT (stage_id) DO NOTHING");
	stage_builder.build().execute(&mut *executor).await?;

	if item_records.is_empty() {
		return Ok(());
	}

	let mut item_builder = QueryBuilder::new(
		"\
INSERT INTO search_trace_stage_items (
	id,
	stage_id,
	item_id,
	note_id,
	chunk_id,
	metrics
) ",
	);

	item_builder.push_values(item_records, |mut b, (stage_id, item)| {
		b.push_bind(item.id)
			.push_bind(stage_id)
			.push_bind(item.item_id)
			.push_bind(item.note_id)
			.push_bind(item.chunk_id)
			.push_bind(item.metrics);
	});
	item_builder.push(" ON CONFLICT (id) DO NOTHING");
	item_builder.build().execute(executor).await?;

	Ok(())
}
