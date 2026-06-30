use crate::worker::{PgExecutor, Result, TraceRecord, Uuid, Value};

pub(in crate::worker::trace_jobs) async fn insert_trace_tx<'e, E>(
	executor: E,
	trace_id: Uuid,
	trace: &TraceRecord,
	expanded_queries_json: Value,
	allowed_scopes_json: Value,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"INSERT INTO search_traces (
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	expansion_mode,
	expanded_queries,
	allowed_scopes,
	candidate_count,
	top_k,
	config_snapshot,
	trace_version,
	created_at,
	expires_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15
)
ON CONFLICT (trace_id) DO NOTHING",
	)
	.bind(trace_id)
	.bind(trace.tenant_id.as_str())
	.bind(trace.project_id.as_str())
	.bind(trace.agent_id.as_str())
	.bind(trace.read_profile.as_str())
	.bind(trace.query.as_str())
	.bind(trace.expansion_mode.as_str())
	.bind(expanded_queries_json)
	.bind(allowed_scopes_json)
	.bind(trace.candidate_count as i32)
	.bind(trace.top_k as i32)
	.bind(trace.config_snapshot.clone())
	.bind(trace.trace_version)
	.bind(trace.created_at)
	.bind(trace.expires_at)
	.execute(executor)
	.await?;

	Ok(())
}
