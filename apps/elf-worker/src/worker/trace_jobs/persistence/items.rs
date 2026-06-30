use crate::worker::{PgExecutor, QueryBuilder, Result, TraceItemInsert, TraceItemRecord, Uuid};

pub(in crate::worker::trace_jobs) async fn insert_trace_items_tx<'e, E>(
	executor: E,
	trace_id: Uuid,
	items: Vec<TraceItemRecord>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if items.is_empty() {
		return Ok(());
	}

	let mut inserts = Vec::with_capacity(items.len());

	for item in items {
		inserts.push(TraceItemInsert {
			item_id: item.item_id,
			note_id: item.note_id,
			chunk_id: item.chunk_id,
			rank: item.rank as i32,
			final_score: item.final_score,
			explain: item.explain,
		});
	}

	let mut builder = QueryBuilder::new(
		"\
INSERT INTO search_trace_items (
	item_id,
	trace_id,
	note_id,
	chunk_id,
	rank,
	final_score,
	explain
) ",
	);

	builder.push_values(inserts, |mut b, item| {
		b.push_bind(item.item_id)
			.push_bind(trace_id)
			.push_bind(item.note_id)
			.push_bind(item.chunk_id)
			.push_bind(item.rank)
			.push_bind(item.final_score)
			.push_bind(item.explain);
	});
	builder.push(" ON CONFLICT (item_id) DO NOTHING");
	builder.build().execute(executor).await?;

	Ok(())
}
