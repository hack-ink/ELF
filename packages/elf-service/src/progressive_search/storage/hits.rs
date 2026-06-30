use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result,
	progressive_search::{storage::hash, types::HitItem},
};
use elf_domain::english_gate;

pub(crate) async fn record_detail_hits<'e, E>(
	executor: E,
	query: &str,
	items: &[HitItem],
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if !english_gate::is_english_natural_language(query) {
		return Err(Error::NonEnglishInput { field: "$.query".to_string() });
	}

	let query_hash = hash::hash_query(query);
	let mut hit_ids = Vec::with_capacity(items.len());
	let mut note_ids = Vec::with_capacity(items.len());
	let mut chunk_ids = Vec::with_capacity(items.len());
	let mut ranks = Vec::with_capacity(items.len());
	let mut final_scores = Vec::with_capacity(items.len());

	for item in items {
		let rank = i32::try_from(item.rank).map_err(|_| Error::InvalidRequest {
			message: "Search session rank is out of range.".to_string(),
		})?;

		hit_ids.push(Uuid::new_v4());
		note_ids.push(item.note_id);
		chunk_ids.push(item.chunk_id);
		ranks.push(rank);
		final_scores.push(item.final_score);
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
