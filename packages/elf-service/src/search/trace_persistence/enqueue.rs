use crate::{
	Error,
	search::{OffsetDateTime, PgExecutor, Result, TracePayload, Uuid},
};

pub(in crate::search) async fn enqueue_trace<'e, E>(
	executor: E,
	payload: TracePayload,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let now = OffsetDateTime::now_utc();
	let payload_json = serde_json::to_value(&payload).map_err(|err| Error::Storage {
		message: format!("Failed to encode search trace payload: {err}"),
	})?;

	sqlx::query(
		"\
INSERT INTO search_trace_outbox (
	outbox_id,
	trace_id,
	status,
	attempts,
	last_error,
	available_at,
	payload,
	created_at,
	updated_at
)
VALUES ($1, $2, 'PENDING', 0, NULL, $3, $4, $3, $3)",
	)
	.bind(Uuid::new_v4())
	.bind(payload.trace.trace_id)
	.bind(now)
	.bind(payload_json)
	.execute(executor)
	.await?;

	Ok(())
}
