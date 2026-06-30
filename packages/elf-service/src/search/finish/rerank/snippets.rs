use crate::search::{
	self, ChunkCandidate, ChunkMeta, ChunkSnippet, ElfService, HashMap, NoteMeta, Result, Uuid,
	ranking,
};

impl ElfService {
	pub(in crate::search) async fn build_snippet_items(
		&self,
		filtered_candidates: &[ChunkCandidate],
		note_meta: &HashMap<Uuid, NoteMeta>,
	) -> Result<Vec<ChunkSnippet>> {
		if filtered_candidates.is_empty() {
			return Ok(Vec::new());
		}

		let pairs = ranking::collect_neighbor_pairs(filtered_candidates);
		let chunk_rows = search::fetch_chunks_by_pair(&self.db.pool, &pairs).await?;
		let mut chunk_by_id = HashMap::new();
		let mut chunk_by_note_index = HashMap::new();

		for row in chunk_rows {
			chunk_by_note_index.insert((row.note_id, row.chunk_index), row.clone());
			chunk_by_id.insert(row.chunk_id, row);
		}

		let mut items = Vec::new();

		for candidate in filtered_candidates {
			let Some(chunk_row) = chunk_by_id.get(&candidate.chunk_id) else {
				tracing::warn!(
					chunk_id = %candidate.chunk_id,
					"Chunk metadata missing for candidate."
				);

				continue;
			};
			let snippet = ranking::stitch_snippet(
				candidate.note_id,
				chunk_row.chunk_index,
				&chunk_by_note_index,
			);

			if snippet.is_empty() {
				continue;
			}

			let Some(note) = note_meta.get(&candidate.note_id) else { continue };
			let chunk = ChunkMeta {
				chunk_id: chunk_row.chunk_id,
				chunk_index: chunk_row.chunk_index,
				start_offset: chunk_row.start_offset,
				end_offset: chunk_row.end_offset,
			};

			items.push(ChunkSnippet {
				note: note.clone(),
				chunk,
				snippet,
				retrieval_rank: candidate.retrieval_rank,
				retrieval_score: candidate.retrieval_score,
			});
		}

		Ok(items)
	}
}
