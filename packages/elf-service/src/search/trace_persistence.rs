mod candidates;
mod enqueue;
mod header;
mod items;
mod stages;

pub(super) use self::enqueue::enqueue_trace;

use crate::search::{PgConnection, Result, TracePayload};

pub(super) async fn persist_trace_inline(
	executor: &mut PgConnection,
	payload: TracePayload,
) -> Result<()> {
	let trace = payload.trace;
	let items = payload.items;
	let candidates = payload.candidates;
	let stages = payload.stages;
	let trace_id = trace.trace_id;

	header::persist_trace_inline_header(executor, &trace).await?;
	items::persist_trace_inline_items(executor, trace_id, items).await?;
	stages::persist_trace_inline_stages(executor, trace_id, stages).await?;
	candidates::persist_trace_inline_candidates(executor, trace_id, candidates).await?;

	Ok(())
}
