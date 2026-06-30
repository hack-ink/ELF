use crate::worker::{
	PgExecutor, QueryBuilder, Result, TraceCandidateInsert, TraceCandidateRecord, Uuid,
};

pub(in crate::worker::trace_jobs) async fn insert_trace_candidates_tx<'e, E>(
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
