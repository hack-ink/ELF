mod cleanup;

pub(super) use cleanup::{
	purge_expired_cache, purge_expired_search_sessions, purge_expired_trace_candidates,
	purge_expired_traces,
};

use crate::worker::{
	self, Db, PgConnection, PgExecutor, QueryBuilder, Result, TraceCandidateInsert,
	TraceCandidateRecord, TraceItemInsert, TraceItemRecord, TraceOutboxJob, TracePayload,
	TraceRecord, TraceStageInsert, TraceStageItemInsert, TraceTrajectoryStageRecord, Uuid, Value,
};

pub(super) async fn handle_trace_job(db: &Db, job: &TraceOutboxJob) -> Result<()> {
	let payload: TracePayload = serde_json::from_value(job.payload.clone())?;
	let TracePayload { trace, items, candidates, stages } = payload;
	let trace_id = trace.trace_id;
	let expanded_queries_json = worker::encode_json(&trace.expanded_queries, "expanded_queries")?;
	let allowed_scopes_json = worker::encode_json(&trace.allowed_scopes, "allowed_scopes")?;
	let mut tx = db.pool.begin().await?;

	insert_trace_tx(&mut *tx, trace_id, &trace, expanded_queries_json, allowed_scopes_json).await?;
	insert_trace_items_tx(&mut *tx, trace_id, items).await?;
	insert_trace_stages_tx(&mut tx, trace_id, stages).await?;
	insert_trace_candidates_tx(&mut *tx, trace_id, candidates).await?;

	tx.commit().await?;

	Ok(())
}

pub(super) async fn insert_trace_stages_tx(
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

pub(super) async fn insert_trace_tx<'e, E>(
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

pub(super) async fn insert_trace_items_tx<'e, E>(
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

pub(super) async fn insert_trace_candidates_tx<'e, E>(
	executor: E,
	trace_id: Uuid,
	candidates: Vec<TraceCandidateRecord>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if candidates.is_empty() {
		return Ok(());
	}

	let mut inserts = Vec::with_capacity(candidates.len());

	for candidate in candidates {
		inserts.push(TraceCandidateInsert {
			candidate_id: candidate.candidate_id,
			note_id: candidate.note_id,
			chunk_id: candidate.chunk_id,
			chunk_index: candidate.chunk_index,
			snippet: candidate.snippet,
			candidate_snapshot: candidate.candidate_snapshot,
			retrieval_rank: candidate.retrieval_rank as i32,
			rerank_score: candidate.rerank_score,
			note_scope: candidate.note_scope,
			note_importance: candidate.note_importance,
			note_updated_at: candidate.note_updated_at,
			note_hit_count: candidate.note_hit_count,
			note_last_hit_at: candidate.note_last_hit_at,
			created_at: candidate.created_at,
			expires_at: candidate.expires_at,
		});
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

	builder.push_values(inserts, |mut b, candidate| {
		b.push_bind(candidate.candidate_id)
			.push_bind(trace_id)
			.push_bind(candidate.note_id)
			.push_bind(candidate.chunk_id)
			.push_bind(candidate.chunk_index)
			.push_bind(candidate.snippet)
			.push_bind(candidate.candidate_snapshot)
			.push_bind(candidate.retrieval_rank)
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
