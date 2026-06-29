use super::*;

pub(super) async fn record_hits<'e, E>(
	executor: E,
	query: &str,
	scored: &[ScoredChunk],
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if scored.is_empty() {
		return Ok(());
	}

	let query_hash = ranking::hash_query(query);
	let mut hit_ids = Vec::with_capacity(scored.len());
	let mut note_ids = Vec::with_capacity(scored.len());
	let mut chunk_ids = Vec::with_capacity(scored.len());
	let mut ranks = Vec::with_capacity(scored.len());
	let mut final_scores = Vec::with_capacity(scored.len());

	for (rank, scored_chunk) in scored.iter().enumerate() {
		hit_ids.push(Uuid::new_v4());
		note_ids.push(scored_chunk.item.note.note_id);
		chunk_ids.push(scored_chunk.item.chunk.chunk_id);
		ranks.push(rank as i32);
		final_scores.push(scored_chunk.final_score);
	}

	sqlx::query(
		"\
WITH hits AS (
	SELECT *
	FROM unnest(
		$1::uuid[],
		$2::uuid[],
		$3::uuid[],
		$4::int4[],
		$5::real[]
	) AS t(hit_id, note_id, chunk_id, rank, final_score)
),
updated AS (
	UPDATE memory_notes
	SET
		hit_count = hit_count + 1,
		last_hit_at = $6
	WHERE note_id = ANY($2)
)
INSERT INTO memory_hits (
	hit_id,
	note_id,
	chunk_id,
	query_hash,
	rank,
	final_score,
	ts
)
SELECT
	hit_id,
	note_id,
	chunk_id,
	$7,
	rank,
	final_score,
	$6
	FROM hits",
	)
	.bind(&hit_ids)
	.bind(&note_ids)
	.bind(&chunk_ids)
	.bind(&ranks)
	.bind(&final_scores)
	.bind(now)
	.bind(query_hash.as_str())
	.execute(executor)
	.await?;

	Ok(())
}
