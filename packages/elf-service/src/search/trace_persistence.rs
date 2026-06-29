use super::*;

pub(super) async fn enqueue_trace<'e, E>(executor: E, payload: TracePayload) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let now = OffsetDateTime::now_utc();
	let payload_json = serde_json::to_value(&payload).map_err(|err| crate::Error::Storage {
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

pub(super) async fn persist_trace_inline(
	executor: &mut PgConnection,
	payload: TracePayload,
) -> Result<()> {
	let trace = payload.trace;
	let items = payload.items;
	let candidates = payload.candidates;
	let stages = payload.stages;
	let trace_id = trace.trace_id;

	persist_trace_inline_header(executor, &trace).await?;
	persist_trace_inline_items(executor, trace_id, items).await?;
	persist_trace_inline_stages(executor, trace_id, stages).await?;
	persist_trace_inline_candidates(executor, trace_id, candidates).await?;

	Ok(())
}

pub(super) async fn persist_trace_inline_stages(
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

pub(super) async fn persist_trace_inline_header(
	executor: &mut PgConnection,
	trace: &TraceRecord,
) -> Result<()> {
	let expanded_queries_json = serde_json::to_value(&trace.expanded_queries).map_err(|err| {
		crate::Error::Storage { message: format!("Failed to encode expanded_queries: {err}") }
	})?;
	let allowed_scopes_json = serde_json::to_value(&trace.allowed_scopes).map_err(|err| {
		crate::Error::Storage { message: format!("Failed to encode allowed_scopes: {err}") }
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

pub(super) async fn persist_trace_inline_items(
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

pub(super) async fn persist_trace_inline_candidates(
	executor: &mut PgConnection,
	trace_id: Uuid,
	candidates: Vec<TraceCandidateRecord>,
) -> Result<()> {
	if candidates.is_empty() {
		return Ok(());
	}

	let mut builder = QueryBuilder::new(
		"\
INSERT INTO search_trace_candidates (
	candidate_id,
	trace_id,
	note_id,
	chunk_id,
	chunk_index,
	snippet,
	candidate_snapshot,
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at,
	note_hit_count,
	note_last_hit_at,
	created_at,
	expires_at
) ",
	);

	builder.push_values(candidates, |mut b, candidate| {
		b.push_bind(candidate.candidate_id)
			.push_bind(trace_id)
			.push_bind(candidate.note_id)
			.push_bind(candidate.chunk_id)
			.push_bind(candidate.chunk_index)
			.push_bind(candidate.snippet)
			.push_bind(candidate.candidate_snapshot)
			.push_bind(candidate.retrieval_rank as i32)
			.push_bind(candidate.rerank_score)
			.push_bind(candidate.note_scope)
			.push_bind(candidate.note_importance)
			.push_bind(candidate.note_updated_at)
			.push_bind(candidate.note_hit_count)
			.push_bind(candidate.note_last_hit_at)
			.push_bind(candidate.created_at)
			.push_bind(candidate.expires_at);
	});
	builder.push(" ON CONFLICT (candidate_id) DO NOTHING");
	builder.build().execute(executor).await?;

	Ok(())
}
