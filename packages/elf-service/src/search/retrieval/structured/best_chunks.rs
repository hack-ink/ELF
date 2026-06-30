use crate::search::{BestChunkForNoteRow, ElfService, HashMap, Result, Uuid};

impl ElfService {
	pub(in crate::search::retrieval) async fn fetch_best_chunks_for_notes(
		&self,
		embed_version: &str,
		ordered_note_ids: &[Uuid],
		vec_text: &str,
	) -> Result<HashMap<Uuid, (Uuid, i32)>> {
		let best_chunks = sqlx::query_as::<_, BestChunkForNoteRow>(
			"\
SELECT DISTINCT ON (c.note_id)
	c.note_id,
	c.chunk_id,
	c.chunk_index
FROM memory_note_chunks c
JOIN note_chunk_embeddings e
	ON e.chunk_id = c.chunk_id
	AND e.embedding_version = $1
WHERE c.note_id = ANY($2::uuid[])
ORDER BY c.note_id ASC, e.vec <=> $3::text::vector ASC",
		)
		.bind(embed_version)
		.bind(ordered_note_ids)
		.bind(vec_text)
		.fetch_all(&self.db.pool)
		.await?;
		let mut best_by_note = HashMap::new();

		for row in best_chunks {
			best_by_note.insert(row.note_id, (row.chunk_id, row.chunk_index));
		}

		Ok(best_by_note)
	}
}
