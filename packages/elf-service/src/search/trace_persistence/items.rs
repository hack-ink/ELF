use crate::search::{PgConnection, QueryBuilder, Result, TraceItemRecord, Uuid};

pub(in crate::search::trace_persistence) async fn persist_trace_inline_items(
	executor: &mut PgConnection,
	trace_id: Uuid,
	items: Vec<TraceItemRecord>,
) -> Result<()> {
	if items.is_empty() {
		return Ok(());
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

	builder.push_values(items, |mut b, item| {
		let explain_json =
			serde_json::to_value(item.explain).expect("SearchExplain must be JSON-serializable.");

		b.push_bind(item.item_id)
			.push_bind(trace_id)
			.push_bind(item.note_id)
			.push_bind(item.chunk_id)
			.push_bind(item.rank as i32)
			.push_bind(item.final_score)
			.push_bind(explain_json);
	});

	builder.push(" ON CONFLICT (item_id) DO NOTHING");
	builder.build().execute(executor).await?;

	Ok(())
}
