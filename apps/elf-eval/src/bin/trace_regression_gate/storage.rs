use color_eyre::Result;
use uuid::Uuid;

use crate::rows::{CandidateRow, TraceItemRow, TraceRow};
use elf_storage::db::Db;

pub(super) async fn fetch_trace_row(db: &Db, trace_id: &Uuid) -> Result<TraceRow> {
	let row: TraceRow = sqlx::query_as::<_, TraceRow>(
		"\
SELECT
	trace_id,
	query,
	candidate_count,
	top_k,
	created_at
FROM search_traces
WHERE trace_id = $1",
	)
	.bind(trace_id)
	.fetch_one(&db.pool)
	.await?;

	Ok(row)
}

pub(super) async fn fetch_baseline_items(
	db: &Db,
	trace_id: &Uuid,
	top_k: u32,
) -> Result<Vec<TraceItemRow>> {
	let rows: Vec<TraceItemRow> = sqlx::query_as::<_, TraceItemRow>(
		"\
SELECT
	note_id
FROM search_trace_items
WHERE trace_id = $1
ORDER BY rank ASC
LIMIT $2",
	)
	.bind(trace_id)
	.bind(i64::from(top_k.max(1)))
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

pub(super) async fn fetch_candidate_rows(db: &Db, trace_id: &Uuid) -> Result<Vec<CandidateRow>> {
	let rows: Vec<CandidateRow> = sqlx::query_as::<_, CandidateRow>(
		"\
SELECT
	candidate_snapshot,
	note_id,
	chunk_id,
	chunk_index,
	snippet,
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at,
	note_hit_count,
	note_last_hit_at
FROM search_trace_candidates
WHERE trace_id = $1
ORDER BY retrieval_rank ASC",
	)
	.bind(trace_id)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}
