use crate::{
	Error,
	search::{PgConnection, Result, TraceRecord},
};

pub(in crate::search::trace_persistence) async fn persist_trace_inline_header(
	executor: &mut PgConnection,
	trace: &TraceRecord,
) -> Result<()> {
	let expanded_queries_json = serde_json::to_value(&trace.expanded_queries).map_err(|err| {
		Error::Storage { message: format!("Failed to encode expanded_queries: {err}") }
	})?;
	let allowed_scopes_json = serde_json::to_value(&trace.allowed_scopes).map_err(|err| {
		Error::Storage { message: format!("Failed to encode allowed_scopes: {err}") }
	})?;

	sqlx::query(
		"\
INSERT INTO search_traces (
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
	.bind(trace.trace_id)
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
