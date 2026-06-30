use crate::worker::{
	PgConnection, QueryBuilder, Result, TraceStageInsert, TraceStageItemInsert,
	TraceTrajectoryStageRecord, Uuid,
};

pub(in crate::worker::trace_jobs) async fn insert_trace_stages_tx(
	executor: &mut PgConnection,
	trace_id: Uuid,
	stages: Vec<TraceTrajectoryStageRecord>,
) -> Result<()> {
	if stages.is_empty() {
		return Ok(());
	}

	let mut stage_inserts = Vec::with_capacity(stages.len());
	let mut item_inserts = Vec::new();

	for stage in stages {
		stage_inserts.push(TraceStageInsert {
			stage_id: stage.stage_id,
			stage_order: stage.stage_order as i32,
			stage_name: stage.stage_name,
			stage_payload: stage.stage_payload,
			created_at: stage.created_at,
		});

		for item in stage.items {
			item_inserts.push(TraceStageItemInsert {
				id: item.id,
				stage_id: stage.stage_id,
				item_id: item.item_id,
				note_id: item.note_id,
				chunk_id: item.chunk_id,
				metrics: item.metrics,
			});
		}
	}

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

	stage_builder.push_values(stage_inserts, |mut b, stage| {
		b.push_bind(stage.stage_id)
			.push_bind(trace_id)
			.push_bind(stage.stage_order)
			.push_bind(stage.stage_name)
			.push_bind(stage.stage_payload)
			.push_bind(stage.created_at);
	});
	stage_builder.push(" ON CONFLICT (stage_id) DO NOTHING");
	stage_builder.build().execute(&mut *executor).await?;

	if item_inserts.is_empty() {
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

	item_builder.push_values(item_inserts, |mut b, item| {
		b.push_bind(item.id)
			.push_bind(item.stage_id)
			.push_bind(item.item_id)
			.push_bind(item.note_id)
			.push_bind(item.chunk_id)
			.push_bind(item.metrics);
	});
	item_builder.push(" ON CONFLICT (id) DO NOTHING");
	item_builder.build().execute(executor).await?;

	Ok(())
}
