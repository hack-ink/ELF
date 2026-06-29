use color_eyre::Result;
use uuid::Uuid;

use elf_storage::db::Db;

use super::rows::{CandidateRow, ItemRow, StageItemRow, StageRow, TraceRow};

pub(super) async fn fetch_traces(db: &Db, trace_ids: &[Uuid]) -> Result<Vec<TraceRow>> {
	let rows: Vec<TraceRow> = sqlx::query_as::<_, TraceRow>(
		"\
SELECT
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
FROM search_traces
WHERE trace_id = ANY($1)
ORDER BY trace_id ASC",
	)
	.bind(trace_ids)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

pub(super) async fn fetch_candidates(db: &Db, trace_ids: &[Uuid]) -> Result<Vec<CandidateRow>> {
	let rows: Vec<CandidateRow> = sqlx::query_as::<_, CandidateRow>(
		"\
SELECT
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
FROM search_trace_candidates
WHERE trace_id = ANY($1)
ORDER BY trace_id ASC, retrieval_rank ASC, candidate_id ASC",
	)
	.bind(trace_ids)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

pub(super) async fn fetch_items(db: &Db, trace_ids: &[Uuid]) -> Result<Vec<ItemRow>> {
	let rows: Vec<ItemRow> = sqlx::query_as::<_, ItemRow>(
		"\
SELECT
	item_id,
	trace_id,
	note_id,
	chunk_id,
	rank,
	final_score,
	explain
FROM search_trace_items
WHERE trace_id = ANY($1)
ORDER BY trace_id ASC, rank ASC, item_id ASC",
	)
	.bind(trace_ids)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

pub(super) async fn fetch_stages(db: &Db, trace_ids: &[Uuid]) -> Result<Vec<StageRow>> {
	let rows: Vec<StageRow> = sqlx::query_as::<_, StageRow>(
		"\
SELECT
	stage_id,
	trace_id,
	stage_order,
	stage_name,
	stage_payload,
	created_at
FROM search_trace_stages
WHERE trace_id = ANY($1)
ORDER BY trace_id ASC, stage_order ASC, stage_id ASC",
	)
	.bind(trace_ids)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

pub(super) async fn fetch_stage_items(db: &Db, stage_ids: &[Uuid]) -> Result<Vec<StageItemRow>> {
	if stage_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows: Vec<StageItemRow> = sqlx::query_as::<_, StageItemRow>(
		"\
SELECT
	id,
	stage_id,
	item_id,
	note_id,
	chunk_id,
	metrics
FROM search_trace_stage_items
WHERE stage_id = ANY($1)
ORDER BY stage_id ASC, id ASC",
	)
	.bind(stage_ids)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}
