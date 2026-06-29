use super::*;

pub(super) async fn fetch_chunks_by_pair<'e, E>(
	executor: E,
	pairs: &[(Uuid, i32)],
) -> Result<Vec<ChunkRow>>
where
	E: PgExecutor<'e>,
{
	if pairs.is_empty() {
		return Ok(Vec::new());
	}

	let mut builder = QueryBuilder::new(
		"SELECT chunk_id, note_id, chunk_index, start_offset, end_offset, text \
				FROM memory_note_chunks WHERE ",
	);
	let mut separated = builder.separated(" OR ");

	for (note_id, chunk_index) in pairs {
		separated.push("(");
		separated
			.push_unseparated("note_id = ")
			.push_bind_unseparated(note_id)
			.push_unseparated(" AND chunk_index = ")
			.push_bind_unseparated(chunk_index)
			.push_unseparated(")");
	}

	let query = builder.build_query_as();
	let rows = query.fetch_all(executor).await?;

	Ok(rows)
}

pub(super) async fn fetch_note_vectors_for_diversity<'e, E>(
	executor: E,
	scored: &[ScoredChunk],
) -> Result<HashMap<Uuid, Vec<f32>>>
where
	E: PgExecutor<'e>,
{
	if scored.is_empty() {
		return Ok(HashMap::new());
	}

	let mut note_ids = Vec::new();
	let mut embedding_versions = Vec::new();
	let mut seen = HashSet::new();

	for scored_chunk in scored {
		let note_id = scored_chunk.item.note.note_id;

		if seen.insert(note_id) {
			note_ids.push(note_id);
			embedding_versions.push(scored_chunk.item.note.embedding_version.clone());
		}
	}

	let rows = sqlx::query_as::<_, NoteVectorRow>(
		"\
WITH expected AS (
	SELECT *
	FROM unnest($1::uuid[], $2::text[]) AS t(note_id, embedding_version)
)
SELECT
	e.note_id,
	n.vec::text AS vec_text
FROM expected e
JOIN note_embeddings n
	ON n.note_id = e.note_id
	AND n.embedding_version = e.embedding_version",
	)
	.bind(note_ids.as_slice())
	.bind(embedding_versions.as_slice())
	.fetch_all(executor)
	.await?;
	let mut out = HashMap::new();

	for row in rows {
		let vec = crate::parse_pg_vector(row.vec_text.as_str())?;

		out.insert(row.note_id, vec);
	}

	Ok(out)
}
