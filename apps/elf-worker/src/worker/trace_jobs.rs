mod cleanup;
mod persistence;

pub(super) use cleanup::{
	purge_expired_cache, purge_expired_search_sessions, purge_expired_trace_candidates,
	purge_expired_traces,
};

use crate::worker::{self, Db, Result, TraceOutboxJob, TracePayload};

pub(super) async fn handle_trace_job(db: &Db, job: &TraceOutboxJob) -> Result<()> {
	let payload: TracePayload = serde_json::from_value(job.payload.clone())?;
	let TracePayload { trace, items, candidates, stages } = payload;
	let trace_id = trace.trace_id;
	let expanded_queries_json = worker::encode_json(&trace.expanded_queries, "expanded_queries")?;
	let allowed_scopes_json = worker::encode_json(&trace.allowed_scopes, "allowed_scopes")?;
	let mut tx = db.pool.begin().await?;

	persistence::insert_trace_tx(
		&mut *tx,
		trace_id,
		&trace,
		expanded_queries_json,
		allowed_scopes_json,
	)
	.await?;
	persistence::insert_trace_items_tx(&mut *tx, trace_id, items).await?;
	persistence::insert_trace_stages_tx(&mut tx, trace_id, stages).await?;
	persistence::insert_trace_candidates_tx(&mut *tx, trace_id, candidates).await?;

	tx.commit().await?;

	Ok(())
}
